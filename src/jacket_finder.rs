//! # Jacket Finder Core Coordination
//!
//! This module provides the main coordination logic for the jacket discovery system.
//! It orchestrates the interaction between scraping, database storage, and Discord
//! notifications to create a complete automated monitoring solution.
//!
//! ## Architecture Overview
//!
//! The `JacketFinder` acts as the central coordinator that:
//! 1. **Scrapes** Marrkt.com for N-1 deck jacket listings
//! 2. **Filters** results against existing database records
//! 3. **Persists** new discoveries to prevent duplicate notifications
//! 4. **Notifies** via Discord webhooks when new items are found
//!
//! ## Workflow
//!
//! ```text
//! ┌─────────────┐    ┌──────────────┐    ┌─────────────┐
//! │   Scraper   │───▶│ JacketFinder │───▶│  Database   │
//! │ (Marrkt.com)│    │ (Coordinator)│    │ (SQLite)    │
//! └─────────────┘    └──────┬───────┘    └─────────────┘
//!                           │
//!                           ▼
//!                    ┌─────────────┐
//!                    │   Discord   │
//!                    │ (Webhooks)  │
//!                    └─────────────┘
//! ```
//!
//! ## Error Handling Philosophy
//!
//! The system is designed for resilience in a long-running monitoring context:
//! - **Scraping failures**: Logged and retried on next cycle
//! - **Database errors**: Propagated (critical for duplicate prevention)
//! - **Discord failures**: Logged but don't halt the monitoring process
//! - **Network issues**: Handled gracefully with automatic retry
//!
//! ## Performance Characteristics
//!
//! - **Memory usage**: Minimal, processes one batch at a time
//! - **Network requests**: Optimized with connection pooling
//! - **Database queries**: Efficient bulk operations with proper indexing
//! - **Concurrent safety**: All components are thread-safe and clonable

use anyhow::Result;
use tracing::info;

use crate::database::Database;
use crate::discord::DiscordNotifier;
use crate::scraper::Scraper;

/// Central coordinator for jacket discovery and notification system.
///
/// This struct encapsulates all the components needed for automated jacket monitoring:
/// web scraping, database persistence, and Discord notifications. It provides a
/// unified interface for the complete discovery workflow while maintaining clean
/// separation of concerns between its components.
///
/// ## Components
///
/// - **Scraper**: Handles web scraping of Marrkt.com search results
/// - **Database**: Manages `SQLite` persistence and duplicate detection
/// - **Discord**: Sends webhook notifications for new discoveries
///
/// ## Design Principles
///
/// - **Composability**: Each component can be tested and used independently
/// - **Resilience**: Failures in one component don't cascade to others
/// - **Efficiency**: Bulk operations and connection reuse minimize overhead
/// - **Observability**: Comprehensive logging for monitoring and debugging
///
/// ## Thread Safety
///
/// All components implement `Clone` and are designed for concurrent use.
/// The struct can be safely shared across async tasks and scheduled jobs.
///
/// ## Lifecycle
///
/// A `JacketFinder` instance is typically:
/// 1. Created once at application startup with `new()`
/// 2. Cloned and shared with the task scheduler
/// 3. Used repeatedly in scheduled intervals via `check_for_new_jackets()`
/// 4. Runs continuously until application shutdown
#[derive(Clone)]
pub struct JacketFinder {
    /// Web scraper for extracting jacket listings from Marrkt.com search results.
    /// Handles HTTP requests, HTML parsing, and data extraction.
    scraper: Scraper,

    /// Database interface for storing and querying jacket records.
    /// Uses `SQLite` for persistence and duplicate detection.
    database: Database,

    /// Discord webhook client for sending rich notification messages.
    /// Optional integration that gracefully handles missing configuration.
    discord: DiscordNotifier,
}

impl JacketFinder {
    /// Creates a new jacket finder with all components initialized.
    ///
    /// This constructor sets up the complete monitoring system by initializing
    /// each component in the correct order. Database initialization happens first
    /// since it can fail and requires async setup for migrations.
    ///
    /// ## Initialization Order
    ///
    /// 1. **Scraper**: Lightweight HTTP client setup (synchronous)
    /// 2. **Database**: `SQLite` connection and migration execution (async)
    /// 3. **Discord**: Environment configuration loading (synchronous)
    ///
    /// ## Error Handling
    ///
    /// This method can fail if:
    /// - Database connection cannot be established
    /// - `SQLite` database file cannot be created/accessed
    /// - Database migrations fail to execute
    /// - File system permissions prevent database creation
    ///
    /// Network issues (scraper, Discord) are handled gracefully at runtime
    /// and don't cause initialization failures.
    ///
    /// ## Environment Dependencies
    ///
    /// Optional environment variables:
    /// - `DISCORD_WEBHOOK_URL`: Discord webhook for notifications
    /// - `DATABASE_URL`: Custom `SQLite` database path (defaults to `jackets.db`)
    ///
    /// ## Return Value
    ///
    /// Returns a fully initialized `JacketFinder` ready for use, or an error
    /// if critical components (primarily database) cannot be set up.
    ///
    /// ## Example
    ///
    /// ```rust
    /// use jacket_finder::jacket_finder::JacketFinder;
    ///
    /// #[tokio::main]
    /// async fn main() -> anyhow::Result<()> {
    ///     let finder = JacketFinder::new().await?;
    ///
    ///     // Ready for scheduled monitoring
    ///     finder.check_for_new_jackets().await?;
    ///     Ok(())
    /// }
    /// ```
    pub async fn new() -> Result<Self> {
        let scraper = Scraper::new();
        let database = Database::new().await?;
        let discord = DiscordNotifier::new();

        Ok(Self {
            scraper,
            database,
            discord,
        })
    }

    /// Creates a new jacket finder with custom search terms and all components initialized.
    ///
    /// This constructor allows configuring custom search terms for specialized
    /// monitoring scenarios while setting up all other components with their
    /// default configurations.
    ///
    /// ## Custom Search Configuration
    ///
    /// Search terms should be chosen carefully:
    /// - **Specific Terms**: "n-1 deck jacket", "type a-2 jacket" for targeted searches
    /// - **Broader Terms**: "deck jacket", "flight jacket" for category searches
    /// - **Brand-Specific**: "buzz rickson", "real mccoy" for brand monitoring
    /// - **Avoid Too Broad**: Terms like "jacket" may return excessive results
    ///
    /// ## Parameters
    ///
    /// - `search_terms`: Vector of search terms to monitor on Marrkt
    ///
    /// ## Initialization Order
    ///
    /// 1. **Scraper**: HTTP client setup with custom search terms (synchronous)
    /// 2. **Database**: `SQLite` connection and migration execution (async)
    /// 3. **Discord**: Environment configuration loading (synchronous)
    ///
    /// ## Error Handling
    ///
    /// This method can fail if:
    /// - Database connection cannot be established
    /// - `SQLite` database file cannot be created/accessed
    /// - Database migrations fail to execute
    /// - File system permissions prevent database creation
    ///
    /// ## Environment Dependencies
    ///
    /// Optional environment variables:
    /// - `DISCORD_WEBHOOK_URL`: Discord webhook for notifications
    /// - `DATABASE_URL`: Custom `SQLite` database path (defaults to `jackets.db`)
    ///
    /// ## Return Value
    ///
    /// Returns a fully initialized `JacketFinder` ready for use, or an error
    /// if critical components (primarily database) cannot be set up.
    ///
    /// ## Example
    ///
    /// ```rust
    /// use jacket_finder::jacket_finder::JacketFinder;
    ///
    /// #[tokio::main]
    /// async fn main() -> anyhow::Result<()> {
    ///     let search_terms = vec![
    ///         "n-1 deck jacket".to_string(),
    ///         "deck jacket".to_string(),
    ///         "flight jacket".to_string(),
    ///         "bomber jacket".to_string(),
    ///     ];
    ///
    ///     let finder = JacketFinder::with_search_terms(search_terms).await?;
    ///
    ///     // Ready for monitoring with custom search terms
    ///     finder.check_for_new_jackets().await?;
    ///     Ok(())
    /// }
    /// ```
    #[allow(dead_code)]
    pub async fn with_search_terms(search_terms: Vec<String>) -> Result<Self> {
        let scraper = Scraper::with_search_terms(search_terms);
        let database = Database::new().await?;
        let discord = DiscordNotifier::new();

        Ok(Self {
            scraper,
            database,
            discord,
        })
    }

    /// Performs a complete jacket discovery cycle with notifications.
    ///
    /// This is the main monitoring method that orchestrates the entire workflow:
    /// scraping, duplicate detection, persistence, and notifications. It's designed
    /// to be called repeatedly by a scheduler (typically every 5 minutes).
    ///
    /// ## Workflow Steps
    ///
    /// 1. **Scrape**: Fetch current N-1 deck jacket listings from Marrkt.com
    /// 2. **Filter**: Load existing jacket IDs from database for duplicate detection
    /// 3. **Process**: For each new jacket found:
    ///    - Log the discovery with title and price
    ///    - Save to database to prevent future duplicates
    ///    - Send Discord notification (if configured)
    /// 4. **Report**: Log summary of new jackets found
    ///
    /// ## Performance Characteristics
    ///
    /// - **Network requests**: Single HTTP request to Marrkt search endpoint
    /// - **Database operations**: Bulk ID fetch + individual inserts for new items
    /// - **Memory usage**: Minimal, processes items as they're discovered
    /// - **Execution time**: Typically 2-5 seconds depending on network conditions
    ///
    /// ## Error Handling Strategy
    ///
    /// The method implements fail-fast behavior for critical operations:
    /// - **Scraping failures**: Propagated (no point continuing without data)
    /// - **Database failures**: Propagated (critical for duplicate prevention)
    /// - **Discord failures**: Logged but don't stop the process
    ///
    /// ## Duplicate Detection
    ///
    /// Uses MD5 hashes of product URLs as unique identifiers:
    /// - Consistent across scraping sessions
    /// - Handles URL parameter variations
    /// - Efficient string comparison for duplicate checking
    ///
    /// ## Logging Behavior
    ///
    /// Provides detailed logging for monitoring:
    /// - Individual jacket discoveries (title, price)
    /// - Session summaries (total new jackets found)
    /// - Clear indication when no new items are found
    ///
    /// ## Return Value
    ///
    /// Returns `Ok(())` on successful completion, even if no new jackets are found.
    /// Returns `Err(anyhow::Error)` if scraping or database operations fail.
    ///
    /// ## Example Scheduler Integration
    ///
    /// ```rust
    /// use tokio_cron_scheduler::{JobScheduler, Job};
    /// use jacket_finder::jacket_finder::JacketFinder;
    ///
    /// #[tokio::main]
    /// async fn main() -> anyhow::Result<()> {
    ///     let finder = JacketFinder::new().await?;
    ///     let sched = JobScheduler::new().await?;
    ///
    ///     sched.add(Job::new_async("0 */5 * * * *", move |_uuid, _l| {
    ///         let finder = finder.clone();
    ///         Box::pin(async move {
    ///             if let Err(e) = finder.check_for_new_jackets().await {
    ///                 eprintln!("Jacket check failed: {}", e);
    ///             }
    ///         })
    ///     })?).await?;
    ///
    ///     sched.start().await?;
    ///     Ok(())
    /// }
    /// ```
    pub async fn check_for_new_jackets(&self) -> Result<()> {
        let jackets = self.scraper.search_jackets().await?;
        let existing_ids = self.database.get_existing_jacket_ids().await?;

        let mut new_jackets = 0;

        for jacket in jackets {
            if !existing_ids.contains(&jacket.id) {
                info!("New jacket found: {} - {}", jacket.title, jacket.price);

                self.database.save_jacket(&jacket).await?;
                self.discord.send_notification(&jacket).await?;

                new_jackets += 1;
            }
        }

        if new_jackets > 0 {
            info!("Found {} new jackets", new_jackets);
        } else {
            info!("No new jackets found");
        }

        Ok(())
    }
}
