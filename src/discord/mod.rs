//! # Discord Webhook Integration
//!
//! This module provides Discord webhook functionality for sending rich notifications
//! about newly discovered jacket listings. It creates visually appealing embedded
//! messages with product images, prices, and direct links.
//!
//! ## Features
//!
//! - **Rich Embeds**: Creates Discord embeds with images, colors, and structured fields
//! - **Image Support**: Displays both full-size images and thumbnails
//! - **Error Handling**: Graceful handling of webhook failures and network issues
//! - **Rate Limiting**: Respects Discord's webhook rate limits
//! - **Optional Integration**: Gracefully disables if webhook URL is not configured
//!
//! ## Discord Embed Structure
//!
//! Each notification includes:
//! - **Title**: "🧥 New N-1 Deck Jacket Found!"
//! - **Description**: Brand and product name (e.g., "Mister Freedom - N-1 Deck Jacket")
//! - **Color**: Discord blue (`0x0058_65F2`) for consistent branding
//! - **Image**: Full-size product photo prominently displayed
//! - **Thumbnail**: Small product image in the top-right corner
//! - **Fields**: Structured information (Price, Direct Link)
//! - **Timestamp**: When the jacket was discovered
//!
//! ## Rate Limits
//!
//! Discord webhooks have the following limits:
//! - **Requests**: 30 per minute
//! - **Message Size**: 6000 characters total
//! - **Embeds**: 10 per message (we use 1)
//! - **Fields**: 25 per embed (we use 2)
//!
//! ## Environment Configuration
//!
//! Set `DISCORD_WEBHOOK_URL` environment variable with your webhook URL.
//! If not set, notifications will be disabled but logged.

use anyhow::Result;
use reqwest::Client;
use tracing::{error, info, warn};

use crate::models::{
    DiscordEmbed, DiscordField, DiscordImage, DiscordMessage, DiscordThumbnail, Jacket,
};

/// Discord webhook notification client for jacket discoveries.
///
/// This struct encapsulates the HTTP client and webhook URL needed to send
/// Discord notifications when new jacket listings are found. It provides
/// a clean interface for sending rich embed messages while handling optional
/// configuration gracefully.
///
/// ## Fields
///
/// - `client`: Reusable HTTP client for making webhook requests
/// - `webhook_url`: Optional Discord webhook URL from environment configuration
///
/// ## Design Principles
///
/// - **Optional Integration**: Functions normally even when Discord is not configured
/// - **Graceful Degradation**: Logs warnings but continues operation on failures
/// - **Resource Efficiency**: Reuses HTTP client connections for better performance
/// - **Environment Driven**: Configuration comes entirely from environment variables
///
/// ## Thread Safety
///
/// This struct is `Clone` and can be safely shared across async tasks and threads.
/// The underlying `reqwest::Client` is designed for concurrent use.
pub struct DiscordNotifier {
    /// Reusable HTTP client for making webhook requests to Discord's API.
    /// This client handles connection pooling and HTTP/2 multiplexing automatically.
    client: Client,

    /// Optional webhook URL loaded from `DISCORD_WEBHOOK_URL` environment variable.
    /// If `None`, all notification attempts will be silently skipped with a warning log.
    webhook_url: Option<String>,
}

impl DiscordNotifier {
    /// Creates a new Discord notifier with environment-based configuration.
    ///
    /// This constructor initializes the HTTP client and attempts to load the Discord
    /// webhook URL from the `DISCORD_WEBHOOK_URL` environment variable. If the
    /// environment variable is not set, Discord notifications will be disabled
    /// but the application will continue to function normally.
    ///
    /// ## Environment Variables
    ///
    /// - `DISCORD_WEBHOOK_URL`: Full webhook URL from Discord channel settings
    ///   - Format: `https://discord.com/api/webhooks/{id}/{token}`
    ///   - Required for Discord notifications to function
    ///   - If missing, notifications are disabled with a warning log
    ///
    /// ## HTTP Client Configuration
    ///
    /// The internal `reqwest::Client` is configured with default settings:
    /// - Automatic HTTP/2 support when available
    /// - Connection pooling and keep-alive
    /// - Automatic JSON serialization/deserialization
    /// - Standard timeout and retry behavior
    ///
    /// ## Return Value
    ///
    /// Returns a new `DiscordNotifier` instance ready for use. The instance
    /// will function correctly regardless of whether Discord is configured.
    ///
    /// ## Example
    ///
    /// ```rust
    /// use jacket_finder::discord::DiscordNotifier;
    ///
    /// // Create notifier - works with or without DISCORD_WEBHOOK_URL
    /// let notifier = DiscordNotifier::new();
    ///
    /// // Can be used immediately, will log warning if unconfigured
    /// ```
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

    /// Sends a Discord notification for a newly discovered jacket.
    ///
    /// This method creates a rich Discord embed message with the jacket's details
    /// and sends it to the configured webhook URL. If no webhook URL is configured,
    /// the method returns successfully without sending anything.
    ///
    /// ## Parameters
    ///
    /// - `jacket`: Reference to the jacket data to include in the notification
    ///
    /// ## Discord Embed Format
    ///
    /// The generated embed includes:
    /// - **Title**: "🧥 New N-1 Deck Jacket Found!" with jacket emoji
    /// - **Description**: Complete jacket title (brand + product name)
    /// - **URL**: Direct link to the product listing on Marrkt
    /// - **Color**: Discord blue (`0x0058_65F2`) for consistent branding
    /// - **Timestamp**: ISO 8601 formatted discovery time
    /// - **Thumbnail**: Small product image in top-right corner
    /// - **Image**: Full-size product photo prominently displayed
    /// - **Fields**: Structured data (Price and clickable link)
    ///
    /// ## Network Behavior
    ///
    /// - Uses HTTP POST request with JSON payload
    /// - Leverages connection pooling for efficiency
    /// - Respects Discord's webhook rate limits (30 requests/minute)
    /// - Automatically handles HTTP/2 when supported by Discord
    ///
    /// ## Error Handling
    ///
    /// This method implements graceful error handling:
    /// - **Missing webhook URL**: Silent skip with info log
    /// - **Network failures**: Propagated as `anyhow::Error`
    /// - **HTTP errors**: Logged with status code, method continues
    /// - **Serialization errors**: Propagated (should not occur with valid models)
    ///
    /// ## Return Value
    ///
    /// Returns `Ok(())` on success or when Discord is disabled.
    /// Returns `Err(anyhow::Error)` for network or serialization failures.
    ///
    /// ## Example
    ///
    /// ```rust
    /// use jacket_finder::discord::DiscordNotifier;
    /// use jacket_finder::models::Jacket;
    ///
    /// let notifier = DiscordNotifier::new();
    /// let jacket = Jacket {
    ///     id: "abc123".to_string(),
    ///     title: "Vintage N-1 Deck Jacket".to_string(),
    ///     brand: "Buzz Rickson's".to_string(),
    ///     price: "$450".to_string(),
    ///     url: "https://marrkt.com/item/abc123".to_string(),
    ///     image_url: Some("https://marrkt.com/images/abc123.jpg".to_string()),
    ///     discovered_at: chrono::Utc::now(),
    /// };
    ///
    /// // Send notification (graceful if Discord not configured)
    /// notifier.send_notification(&jacket).await?;
    /// ```
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

/// Manual implementation of `Clone` for `DiscordNotifier`.
///
/// This implementation allows the notifier to be cloned and shared across
/// multiple async tasks or threads. The underlying `reqwest::Client` is
/// designed to be cloned efficiently and shares connection pools.
///
/// ## Usage
///
/// Cloning is useful when the same notifier needs to be used in multiple
/// contexts, such as sharing between the main scheduler and error handlers.
///
/// ## Performance
///
/// Cloning a `DiscordNotifier` is lightweight:
/// - The `reqwest::Client` uses `Arc` internally for efficient cloning
/// - The webhook URL is a simple `Option<String>` clone
/// - No network connections are duplicated
impl Clone for DiscordNotifier {
    fn clone(&self) -> Self {
        Self {
            client: self.client.clone(),
            webhook_url: self.webhook_url.clone(),
        }
    }
}
