//! # Jacket Finder Bot
//!
//! A Rust-based bot that monitors [Marrkt](https://www.marrkt.com) for new N-1 deck jacket listings
//! and sends Discord notifications when new items are discovered.
//!
//! ## Features
//!
//! - 🔍 **Intelligent Scraping**: Searches Marrkt every 5 minutes for N-1 deck jacket listings
//! - 💾 **Duplicate Prevention**: Uses `SQLite` database to track seen jackets and avoid duplicate notifications
//! - 🚀 **Discord Integration**: Sends rich notifications with product images, prices, and direct links
//! - 📊 **Comprehensive Logging**: Detailed logging of all bot activities and errors
//! - 🔧 **Modular Architecture**: Clean separation of concerns with dedicated modules for each functionality
//!
//! ## Architecture
//!
//! The bot is structured into several focused modules:
//! - [`models`]: Data structures for jackets and Discord messages
//! - [`database`]: `SQLite` operations and schema management with migrations
//! - [`scraper`]: Web scraping logic for Marrkt product pages
//! - [`discord`]: Discord webhook notifications with rich embeds
//! - [`jacket_finder`]: Main coordination logic that orchestrates all components
//!
//! ## Environment Variables
//!
//! - `DISCORD_WEBHOOK_URL`: Discord webhook URL for notifications (optional)
//!
//! ## Example Usage
//!
//! ```bash
//! # Set up Discord webhook
//! export DISCORD_WEBHOOK_URL="https://discord.com/api/webhooks/..."
//!
//! # Run the bot
//! cargo run
//! ```
//!
//! The bot will:
//! 1. Initialize database and create tables if needed
//! 2. Run an immediate search to populate the database with existing jackets
//! 3. Set up a scheduler to check every 5 minutes for new listings
//! 4. Send Discord notifications only for genuinely new jackets

use anyhow::Result;
use tokio_cron_scheduler::{Job, JobScheduler};
use tracing::{error, info};

mod database;
mod discord;
mod jacket_finder;
mod models;
mod scraper;

use jacket_finder::JacketFinder;

/// Main entry point for the Jacket Finder Bot.
///
/// This function sets up and runs the complete bot lifecycle:
///
/// ## Setup Phase
/// 1. **Environment Loading**: Loads `.env` file variables using `dotenvy`
/// 2. **Logging Initialization**: Sets up `tracing` for structured logging
/// 3. **Component Initialization**: Creates the `JacketFinder` with all dependencies
///
/// ## Initial Check
/// Performs an immediate search to populate the database with existing jackets,
/// preventing duplicate notifications on first run.
///
/// ## Scheduler Setup
/// - Creates a cron scheduler that runs every 5 minutes (`0 */5 * * * *`)
/// - Each scheduled run checks for new jackets and sends notifications
/// - Uses async job execution to prevent blocking
///
/// ## Keep-Alive Loop
/// Maintains the program in a running state with 30-second sleep intervals,
/// allowing the background scheduler to continue operation.
///
/// # Returns
///
/// - `Ok(())`: Never returned in practice as the loop runs indefinitely
/// - `Err`: If critical initialization fails (database, scheduler setup)
///
/// # Environment Variables
///
/// - `DISCORD_WEBHOOK_URL`: Optional Discord webhook for notifications
///
/// # Examples
///
/// ```bash
/// # Run with Discord notifications
/// DISCORD_WEBHOOK_URL="https://discord.com/api/webhooks/..." cargo run
///
/// # Run without notifications (logging only)
/// cargo run
/// ```
#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt::init();

    info!("Starting N-1 Deck Jacket Finder Bot");

    let finder = JacketFinder::new().await?;

    // Run once immediately to test
    if let Err(e) = finder.check_for_new_jackets().await {
        error!("Error during initial check: {}", e);
    }

    // Set up scheduler to run every 5 minutes
    let sched = JobScheduler::new().await?;

    let job_finder = finder.clone();
    sched
        .add(Job::new_async("0 */5 * * * *", move |_uuid, _l| {
            let finder = job_finder.clone();
            Box::pin(async move {
                if let Err(e) = finder.check_for_new_jackets().await {
                    error!("Error checking for jackets: {}", e);
                }
            })
        })?)
        .await?;

    info!("Scheduler started - checking every 5 minutes");
    sched.start().await?;

    // Keep the program running
    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
    }
}
