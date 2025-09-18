//! Core coordination logic for jacket discovery system.
//! 
//! Orchestrates scraping, database operations, and Discord notifications.

use anyhow::Result;
use std::sync::Arc;
use tracing::info;

use crate::database::Database;
use crate::discord::DiscordNotifier;
use crate::scrapers::MarrktScraper;
use crate::traits::WebsiteScraper;

/// Central coordinator for jacket discovery and notifications
#[derive(Clone)]
pub struct JacketFinder {
    scrapers: Vec<Arc<dyn WebsiteScraper>>,
    database: Database,
    discord: DiscordNotifier,
}

impl JacketFinder {
    /// Create a new jacket finder with default scrapers
    /// 
    /// # Returns
    /// * `Result<Self>` - New `JacketFinder` instance or initialization error
    pub async fn new() -> Result<Self> {
        // Initialize default scrapers
        let mut scrapers: Vec<Arc<dyn WebsiteScraper>> = Vec::new();
        
        // Add Marrkt scraper
        let marrkt_scraper = MarrktScraper::new()?;
        scrapers.push(Arc::new(marrkt_scraper));
        
        let database = Database::new().await?;
        let discord = DiscordNotifier::new();

        Ok(Self {
            scrapers,
            database,
            discord,
        })
    }
    
    /// Create a new jacket finder with custom scrapers
    /// 
    /// # Arguments
    /// * `scrapers` - Vector of scrapers to use
    /// 
    /// # Returns
    /// * `Result<Self>` - New `JacketFinder` instance or initialization error
    #[allow(dead_code)]
    pub async fn new_with_scrapers(scrapers: Vec<Arc<dyn WebsiteScraper>>) -> Result<Self> {
        let database = Database::new().await?;
        let discord = DiscordNotifier::new();

        Ok(Self {
            scrapers,
            database,
            discord,
        })
    }
    
    /// Add a scraper to the list of active scrapers
    /// 
    /// # Arguments
    /// * `scraper` - The scraper to add
    #[allow(dead_code)]
    pub fn add_scraper(&mut self, scraper: Arc<dyn WebsiteScraper>) {
        self.scrapers.push(scraper);
    }


    /// Check for new jacket listings across all configured websites and send notifications
    /// 
    /// # Returns
    /// * `Result<()>` - Success or error from scraping, database, or notification operations
    pub async fn check_for_new_jackets(&self) -> Result<()> {
        info!("Starting jacket search across {} websites", self.scrapers.len());
        
        let existing_ids = self.database.get_existing_jacket_ids().await?;
        let mut all_jackets = Vec::new();

        // Search across all configured scrapers
        for scraper in &self.scrapers {
            info!("Searching on website: {}", scraper.config().name);
            
            match scraper.search_jackets().await {
                Ok(jackets) => {
                    info!("Found {} jackets on {}", jackets.len(), scraper.config().name);
                    all_jackets.extend(jackets);
                }
                Err(e) => {
                    // Log error but continue with other scrapers
                    tracing::error!("Error searching on {}: {}", scraper.config().name, e);
                }
            }
        }

        let mut new_jackets = 0;

        for jacket in all_jackets {
            if !existing_ids.contains(&jacket.id) {
                info!("New jacket found: {} - {}", jacket.title, jacket.price);

                self.database.save_jacket(&jacket).await?;
                self.discord.send_notification(&jacket).await?;

                new_jackets += 1;
            }
        }

        if new_jackets > 0 {
            info!("Found {} new jackets across all websites", new_jackets);
        } else {
            info!("No new jackets found across all websites");
        }

        Ok(())
    }
}
