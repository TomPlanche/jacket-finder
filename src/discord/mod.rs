//! Discord webhook notifications for jacket discoveries

use anyhow::Result;
use reqwest::Client;
use tracing::{error, info, warn};

use crate::models::{
    DiscordEmbed, DiscordField, DiscordImage, DiscordMessage, DiscordThumbnail, Jacket,
};

/// Discord webhook client for sending jacket notifications
pub struct DiscordNotifier {
    client: Client,
    webhook_url: Option<String>,
}

impl DiscordNotifier {
    /// Create a new Discord notifier from environment configuration
    /// 
    /// # Returns
    /// * `Self` - New `DiscordNotifier` instance with optional webhook configuration
    pub fn new() -> Self {
        let client = Client::new();
        let webhook_url = std::env::var("DISCORD_WEBHOOK_URL").ok();

        if webhook_url.is_none() {
            warn!("DISCORD_WEBHOOK_URL not set - Discord notifications will be disabled");
        }

        Self {
            client,
            webhook_url,
        }
    }

    /// Send a Discord notification for a newly discovered jacket
    /// 
    /// # Arguments
    /// * `jacket` - Reference to the jacket data to include in the notification
    /// 
    /// # Returns
    /// * `Result<()>` - Success or network/serialization error
    pub async fn send_notification(&self, jacket: &Jacket) -> Result<()> {
        if let Some(webhook_url) = &self.webhook_url {
            let embed = DiscordEmbed {
                title: "🧥 New N-1 Deck Jacket Found!".to_string(),
                description: jacket.title.clone(),
                url: jacket.url.clone(),
                color: 0x0058_65F2, // Discord blue
                timestamp: jacket.discovered_at.to_rfc3339(),
                thumbnail: jacket
                    .image_url
                    .as_ref()
                    .map(|url| DiscordThumbnail { url: url.clone() }),
                image: jacket
                    .image_url
                    .as_ref()
                    .map(|url| DiscordImage { url: url.clone() }),
                fields: vec![
                    DiscordField {
                        name: "Price".to_string(),
                        value: jacket.price.clone(),
                        inline: true,
                    },
                    DiscordField {
                        name: "Link".to_string(),
                        value: format!("[View on Marrkt]({})", jacket.url),
                        inline: true,
                    },
                ],
            };

            let message = DiscordMessage {
                embeds: vec![embed],
            };

            let response = self.client.post(webhook_url).json(&message).send().await?;

            if response.status().is_success() {
                info!("Discord notification sent for jacket: {}", jacket.title);
            } else {
                error!("Failed to send Discord notification: {}", response.status());
            }
        }

        Ok(())
    }
}

impl Clone for DiscordNotifier {
    fn clone(&self) -> Self {
        Self {
            client: self.client.clone(),
            webhook_url: self.webhook_url.clone(),
        }
    }
}
