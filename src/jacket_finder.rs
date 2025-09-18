use anyhow::Result;
use tracing::info;

use crate::database::Database;
use crate::discord::DiscordNotifier;
use crate::scraper::Scraper;

#[derive(Clone)]
pub struct JacketFinder {
    scraper: Scraper,
    database: Database,
    discord: DiscordNotifier,
}

impl JacketFinder {
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
