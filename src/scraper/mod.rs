use anyhow::Result;
use chrono::Utc;
use reqwest::Client;
use scraper::{Html, Selector};
use tracing::info;

use crate::models::Jacket;

pub struct Scraper {
    client: Client,
}

impl Scraper {
    pub fn new() -> Self {
        let client = Client::builder()
            .user_agent("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36")
            .build()
            .expect("Failed to create HTTP client");

        Self { client }
    }

    pub async fn search_jackets(&self) -> Result<Vec<Jacket>> {
        info!("Searching for n-1 deck jackets on Marrkt");

        let search_url = "https://www.marrkt.com/search?q=n-1+deck+jacket";

        let response = self.client.get(search_url).send().await?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "Failed to fetch search page: {}",
                response.status()
            ));
        }

        let html = response.text().await?;
        let document = Html::parse_document(&html);

        let mut jackets = Vec::new();

        // Updated selectors based on actual HTML structure
        let product_selector = Selector::parse(".product-card-wrapper").unwrap();
        let title_selector = Selector::parse(".product-title a, .card-title a").unwrap();
        let price_selector = Selector::parse(".product-price-exc-vat").unwrap();
        let brand_selector = Selector::parse(".card-subtitle").unwrap();
        let link_selector = Selector::parse(".product-card a, .card-image a").unwrap();
        let image_selector = Selector::parse(".responsive-image__image").unwrap();

        for product in document.select(&product_selector) {
            if let Some(link) = product.select(&link_selector).next()
                && let Some(href) = link.value().attr("href")
            {
                let url = if href.starts_with("http") {
                    href.to_string()
                } else {
                    format!("https://www.marrkt.com{href}")
                };

                let product_title = product.select(&title_selector).next().map_or_else(
                    || "Unknown Item".to_string(),
                    |el| el.text().collect::<String>().trim().to_string(),
                );

                let brand = product.select(&brand_selector).next().map_or_else(
                    || "Unknown Brand".to_string(),
                    |el| el.text().collect::<String>().trim().to_string(),
                );

                // Combine brand and title for full item name
                let title = format!("{brand} - {product_title}");

                // Only include items that mention "n-1" or "deck jacket"
                let title_lower = title.to_lowercase();
                if !title_lower.contains("n-1") && !title_lower.contains("deck jacket") {
                    continue;
                }

                let price = product.select(&price_selector).next().map_or_else(
                    || "Price not found".to_string(),
                    |el| el.text().collect::<String>().trim().to_string(),
                );

                let image_url = product
                    .select(&image_selector)
                    .next()
                    .and_then(|img| {
                        // Try data-src first (for lazy loading), then src
                        img.value()
                            .attr("data-src")
                            .or_else(|| img.value().attr("src"))
                    })
                    .map(|src| {
                        if src.starts_with("http") {
                            src.to_string()
                        } else if src.starts_with("//") {
                            format!("https:{src}")
                        } else {
                            format!("https://www.marrkt.com{src}")
                        }
                    });

                // Generate a unique ID based on URL
                let id = format!("{:x}", md5::compute(&url));

                let jacket = Jacket {
                    id,
                    title,
                    price,
                    url,
                    image_url,
                    discovered_at: Utc::now(),
                };

                jackets.push(jacket);
            }
        }

        info!("Found {} potential n-1 deck jackets", jackets.len());
        Ok(jackets)
    }
}

impl Clone for Scraper {
    fn clone(&self) -> Self {
        Self {
            client: self.client.clone(),
        }
    }
}
