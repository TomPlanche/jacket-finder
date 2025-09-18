use anyhow::Result;
use tokio_cron_scheduler::{Job, JobScheduler};
use tracing::{error, info};

mod database;
mod discord;
mod jacket_finder;
mod models;
mod scraper;

use jacket_finder::JacketFinder;

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
