use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Jacket {
    pub id: String,
    pub title: String,
    pub price: String,
    pub url: String,
    pub image_url: Option<String>,
    pub discovered_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct DiscordEmbed {
    pub title: String,
    pub description: String,
    pub url: String,
    pub color: u32,
    pub timestamp: String,
    pub thumbnail: Option<DiscordThumbnail>,
    pub fields: Vec<DiscordField>,
}

#[derive(Debug, Serialize)]
pub struct DiscordThumbnail {
    pub url: String,
}

#[derive(Debug, Serialize)]
pub struct DiscordField {
    pub name: String,
    pub value: String,
    pub inline: bool,
}

#[derive(Debug, Serialize)]
pub struct DiscordMessage {
    pub embeds: Vec<DiscordEmbed>,
}
