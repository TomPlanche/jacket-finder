//! N-1 Deck Jacket Finder Bot
//!
//! Monitors Marrkt.com for new N-1 deck jacket listings and sends Discord notifications.
//! Runs every 5 minutes and stores results in `SQLite` to prevent duplicate notifications.

use anyhow::Result;
use tokio_cron_scheduler::{Job, JobScheduler};
use tracing::{error, info};

mod database;
mod discord;
mod jacket_finder;
mod models;
mod scrapers;
mod traits;

/// Application entry point and runtime management
struct App {
    finder: JacketFinder,
    scheduler: JobScheduler,
}

impl App {
    /// Initialize the application with all components
    async fn new() -> Result<Self> {
        let finder = JacketFinder::new().await?;
        let scheduler = JobScheduler::new().await?;
        
        Ok(Self { finder, scheduler })
    }

    /// Set up the job scheduler to run every 5 minutes
    async fn setup_scheduler(&self) -> Result<()> {
        let finder = self.finder.clone();
        self.scheduler
            .add(Job::new_async("0 */5 * * * *", move |_uuid, _l| {
                let finder = finder.clone();
                Box::pin(async move {
                    if let Err(e) = finder.check_for_new_jackets().await {
                        error!("Error checking for jackets: {}", e);
                    }
                })
            })?)
            .await?;
        
        info!("Scheduler configured to run every 5 minutes");
        Ok(())
    }

    /// Run the application
    async fn run(&self) -> Result<()> {
        // Initial check to populate database
        info!("Running initial jacket check");
        if let Err(e) = self.finder.check_for_new_jackets().await {
            error!("Error during initial check: {}", e);
        }

        // Start the scheduler
        self.setup_scheduler().await?;
        self.scheduler.start().await?;
        info!("Jacket finder bot is running");

        // Keep running
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
        }
    }
}

use jacket_finder::JacketFinder;

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt::init();

    info!("Starting N-1 Deck Jacket Finder Bot");
    
    let app = App::new().await?;
    app.run().await
}
