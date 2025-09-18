//! Data models for jacket information and Discord webhook payloads

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A jacket listing scraped from Marrkt
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Jacket {
    pub id: String,
    pub title: String,
    pub price: String,
    pub url: String,
    pub image_url: Option<String>,
    pub discovered_at: DateTime<Utc>,
}

/// Discord embed structure for rich notifications
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

/// Small thumbnail image for Discord embeds
#[derive(Debug, Serialize)]
pub struct DiscordThumbnail {
    pub url: String,
}

/// Full-size image for Discord embeds
#[derive(Debug, Serialize)]
pub struct DiscordImage {
    pub url: String,
}

/// Key-value field for Discord embeds
#[derive(Debug, Serialize)]
pub struct DiscordField {
    pub name: String,
    pub value: String,
    pub inline: bool,
}

/// Discord webhook message payload
#[derive(Debug, Serialize)]
pub struct DiscordMessage {
    pub embeds: Vec<DiscordEmbed>,
}
