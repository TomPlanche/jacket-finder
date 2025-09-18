//! # Data Models
//!
//! This module defines all data structures used throughout the jacket finder bot.
//! It includes both domain models (jacket information) and integration models
//! (Discord webhook payloads).
//!
//! ## Core Models
//!
//! - [`Jacket`]: Represents a scraped jacket listing from Marrkt
//! - [`DiscordMessage`]: Root structure for Discord webhook messages
//! - [`DiscordEmbed`]: Rich embed content for Discord notifications
//! - [`DiscordImage`]: Full-size image attachments for Discord embeds
//! - [`DiscordThumbnail`]: Small thumbnail images for Discord embeds
//! - [`DiscordField`]: Key-value fields within Discord embeds
//!
//! All models are designed to be:
//! - **Serializable**: Can be converted to JSON for API calls or storage
//! - **Cloneable**: Support efficient copying for async operations
//! - **Debug-friendly**: Provide meaningful debug output for troubleshooting

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Represents a scraped N-1 deck jacket listing from Marrkt.
///
/// This is the core domain model that contains all information about a discovered jacket.
/// Each jacket gets a unique identifier to prevent duplicate notifications.
///
/// # Fields
///
/// - `id`: Unique identifier generated from the jacket's URL hash (MD5)
/// - `title`: Combined brand and product name (e.g., "Mister Freedom - N-1 Deck Jacket")
/// - `price`: Price string as displayed on Marrkt (e.g., "€349,95")
/// - `url`: Direct link to the product page on Marrkt
/// - `image_url`: Optional URL to the product image (may be lazy-loaded or protocol-relative)
/// - `discovered_at`: UTC timestamp when the jacket was first found
///
/// # Examples
///
/// ```rust
/// use chrono::Utc;
/// use jacket_finder::models::Jacket;
///
/// let jacket = Jacket {
///     id: "a1b2c3d4".to_string(),
///     title: "Mister Freedom - N-1 Deck Jacket".to_string(),
///     price: "€349,95".to_string(),
///     url: "https://www.marrkt.com/products/n-1-deck-jacket-33".to_string(),
///     image_url: Some("https://cdn.marrkt.com/image.jpg".to_string()),
///     discovered_at: Utc::now(),
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Jacket {
    pub id: String,
    pub title: String,
    pub price: String,
    pub url: String,
    pub image_url: Option<String>,
    pub discovered_at: DateTime<Utc>,
}

/// Rich embed structure for Discord webhook messages.
///
/// Discord embeds provide a visually appealing way to display jacket information
/// with colors, images, and structured fields. This structure follows Discord's
/// webhook API specification.
///
/// # Fields
///
/// - `title`: Main heading for the embed (e.g., "🧥 New N-1 Deck Jacket Found!")
/// - `description`: Primary content, typically the jacket's brand and name
/// - `url`: Clickable link that makes the entire embed link to the product page
/// - `color`: Hex color for the embed's left border (`0x0058_65F2` = Discord blue)
/// - `timestamp`: ISO 8601 formatted timestamp showing when the jacket was discovered
/// - `thumbnail`: Optional small image displayed in the top-right corner
/// - `image`: Optional full-size image displayed prominently in the embed
/// - `fields`: Array of key-value pairs for structured information (price, links)
///
/// # Discord Limits
///
/// - Title: 256 characters max
/// - Description: 4096 characters max
/// - Fields: 25 max per embed
/// - Field names: 256 characters max
/// - Field values: 1024 characters max
///
/// # Examples
///
/// ```rust
/// use jacket_finder::models::{DiscordEmbed, DiscordField};
///
/// let embed = DiscordEmbed {
///     title: "🧥 New N-1 Deck Jacket Found!".to_string(),
///     description: "Mister Freedom - N-1 Deck Jacket".to_string(),
///     url: "https://www.marrkt.com/products/jacket".to_string(),
///     color: 0x0058_65F2,
///     timestamp: "2024-01-01T12:00:00Z".to_string(),
///     thumbnail: None,
///     image: None,
///     fields: vec![
///         DiscordField {
///             name: "Price".to_string(),
///             value: "€349,95".to_string(),
///             inline: true,
///         }
///     ],
/// };
/// ```
#[derive(Debug, Serialize)]
pub struct DiscordEmbed {
    pub title: String,
    pub description: String,
    pub url: String,
    pub color: u32,
    pub timestamp: String,
    pub thumbnail: Option<DiscordThumbnail>,
    pub image: Option<DiscordImage>,
    pub fields: Vec<DiscordField>,
}

/// Small thumbnail image for Discord embeds.
///
/// Thumbnails appear as small images in the top-right corner of Discord embeds.
/// They're typically used alongside or instead of full-size images to provide
/// a visual preview without taking up much space.
///
/// # Fields
///
/// - `url`: Direct HTTP(S) URL to the thumbnail image
///
/// # Constraints
///
/// - Must be a valid HTTP(S) URL
/// - Discord supports common image formats (PNG, JPEG, GIF, WebP)
/// - Recommended size: 80x80 pixels (Discord will resize automatically)
/// - Maximum file size: 8MB
#[derive(Debug, Serialize)]
pub struct DiscordThumbnail {
    pub url: String,
}

/// Full-size image attachment for Discord embeds.
///
/// Images appear as large, prominent visuals within Discord embeds, typically
/// below the description text. They provide the main visual representation
/// of the jacket listing.
///
/// # Fields
///
/// - `url`: Direct HTTP(S) URL to the full-size image
///
/// # Constraints
///
/// - Must be a valid HTTP(S) URL
/// - Discord supports common image formats (PNG, JPEG, GIF, WebP)
/// - Recommended width: 400-500 pixels (Discord will scale appropriately)
/// - Maximum file size: 8MB
/// - Aspect ratio preserved automatically
#[derive(Debug, Serialize)]
pub struct DiscordImage {
    pub url: String,
}

/// Individual field within a Discord embed.
///
/// Fields provide structured key-value information displayed in a grid layout
/// within the embed. They're perfect for showing prices, links, sizes, and
/// other jacket metadata in an organized manner.
///
/// # Fields
///
/// - `name`: The field label/key (e.g., "Price", "Brand", "Size")
/// - `value`: The field content/value (e.g., "€349,95", "Mister Freedom", "Size 38")
/// - `inline`: Whether the field should display inline with others (up to 3 per row)
///
/// # Layout Behavior
///
/// - `inline: true`: Up to 3 fields per row in a grid layout
/// - `inline: false`: Full-width field, forces a new row
/// - Mix of inline/non-inline creates dynamic layouts
///
/// # Examples
///
/// ```rust
/// use jacket_finder::models::DiscordField;
///
/// let price_field = DiscordField {
///     name: "Price".to_string(),
///     value: "€349,95".to_string(),
///     inline: true, // Display alongside other inline fields
/// };
///
/// let description_field = DiscordField {
///     name: "Description".to_string(),
///     value: "Authentic N-1 deck jacket from WWII era...".to_string(),
///     inline: false, // Takes full width
/// };
/// ```
#[derive(Debug, Serialize)]
pub struct DiscordField {
    pub name: String,
    pub value: String,
    pub inline: bool,
}

/// Root structure for Discord webhook messages.
///
/// This is the top-level payload sent to Discord webhooks. It can contain
/// multiple embeds, plain text content, and various other Discord features.
/// For the jacket finder bot, we primarily use single-embed messages.
///
/// # Fields
///
/// - `embeds`: Array of rich embed objects to display
///
/// # Discord Limits
///
/// - Maximum 10 embeds per message
/// - Total message size: 6000 characters (including all embed content)
/// - Webhook rate limit: 30 requests per minute
///
/// # Examples
///
/// ```rust
/// use jacket_finder::models::{DiscordMessage, DiscordEmbed};
///
/// let message = DiscordMessage {
///     embeds: vec![
///         DiscordEmbed {
///             title: "🧥 New Jacket Found!".to_string(),
///             description: "Check out this amazing find!".to_string(),
///             // ... other embed fields
///             # url: "https://example.com".to_string(),
///             # color: 0x0058_65F2,
///             # timestamp: "2024-01-01T12:00:00Z".to_string(),
///             # thumbnail: None,
///             # image: None,
///             # fields: vec![],
///         }
///     ],
/// };
/// ```
#[derive(Debug, Serialize)]
pub struct DiscordMessage {
    pub embeds: Vec<DiscordEmbed>,
}
