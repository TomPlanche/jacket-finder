//! Web scraping for Marrkt jacket listings with configurable search terms

use anyhow::Result;
use chrono::Utc;
use reqwest::Client;
use scraper::{Html, Selector};
use tracing::info;

use crate::models::Jacket;

/// Web scraper for Marrkt jacket listings
pub struct Scraper {
    client: Client,
    search_terms: Vec<String>,
}

impl Scraper {
    /// Create a new scraper with default search terms
    /// 
    /// # Returns
    /// * `Self` - New Scraper instance with default search configuration
    pub fn new() -> Self {
        let client = Client::builder()
            .user_agent("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36")
            .build()
            .expect("Failed to create HTTP client");

        let search_terms = vec!["n-1 deck jacket".to_string(), "deck jacket".to_string()];

        Self {
            client,
            search_terms,
        }
    }


    /// Search for jacket listings on Marrkt and extract product data
    /// 
    /// # Returns
    /// * `Result<Vec<Jacket>>` - Vector of found jackets or scraping error
    #[allow(clippy::too_many_lines)]
    pub async fn search_jackets(&self) -> Result<Vec<Jacket>> {
        info!(
            "Searching for jackets on Marrkt with {} search terms",
            self.search_terms.len()
        );

        let mut all_jackets = std::collections::HashMap::new(); // For deduplication by URL

        // Updated selectors based on actual HTML structure
        let product_selector = Selector::parse(".product-card-wrapper").unwrap();
        let title_selector = Selector::parse(".product-title a, .card-title a").unwrap();
        let price_selector = Selector::parse(".product-price-exc-vat").unwrap();
        let brand_selector = Selector::parse(".card-subtitle").unwrap();
        let link_selector = Selector::parse(".product-card a, .card-image a").unwrap();
        let image_selector = Selector::parse(".responsive-image__image").unwrap();

        for search_term in &self.search_terms {
            info!("Searching for: {}", search_term);

            let encoded_term = urlencoding::encode(search_term);
            let search_url = format!("https://www.marrkt.com/search?q={encoded_term}");

            let response = self.client.get(&search_url).send().await?;

            if !response.status().is_success() {
                return Err(anyhow::anyhow!(
                    "Failed to fetch search page for '{}': {}",
                    search_term,
                    response.status()
                ));
            }

            let html = response.text().await?;
            let document = Html::parse_document(&html);

            for product in document.select(&product_selector) {
                if let Some(link) = product.select(&link_selector).next()
                    && let Some(href) = link.value().attr("href")
                {
                    let mut url = if href.starts_with("http") {
                        href.to_string()
                    } else {
                        format!("https://www.marrkt.com{href}")
                    };

                    // Normalize URL by removing query parameters to avoid duplicates
                    // e.g., "...?_pos=1&_sid=abc" becomes just the base URL
                    if let Some(query_start) = url.find('?') {
                        url.truncate(query_start);
                    }

                    // Skip if we've already processed this normalized URL
                    if all_jackets.contains_key(&url) {
                        continue;
                    }

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

                    // Check if this item matches any of our search terms
                    let title_lower = title.to_lowercase();
                    let matches_search_term = self
                        .search_terms
                        .iter()
                        .any(|term| title_lower.contains(&term.to_lowercase()));

                    if !matches_search_term {
                        continue;
                    }

                    // Skip sold out items
                    let sold_out_selector = Selector::parse(".card-body p").unwrap();
                    let is_sold_out = product
                        .select(&sold_out_selector)
                        .any(|el| el.text().collect::<String>().trim() == "Sold Out");

                    if is_sold_out {
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
                            let mut processed_url = if src.starts_with("http") {
                                src.to_string()
                            } else if src.starts_with("//") {
                                format!("https:{src}")
                            } else {
                                format!("https://www.marrkt.com{src}")
                            };

                            // Replace {width} placeholder with fixed width for Discord display
                            if processed_url.contains("{width}") {
                                processed_url = processed_url.replace("{width}", "800");
                            }

                            processed_url
                        });

                    // Generate a unique ID based on URL
                    let id = format!("{:x}", md5::compute(&url));

                    let jacket = Jacket {
                        id,
                        title,
                        price,
                        url: url.clone(),
                        image_url,
                        discovered_at: Utc::now(),
                    };

                    all_jackets.insert(url, jacket);
                }
            }
        }

        let jackets: Vec<Jacket> = all_jackets.into_values().collect();
        info!(
            "Found {} unique jackets across all search terms",
            jackets.len()
        );
        Ok(jackets)
    }
}

impl Clone for Scraper {
    fn clone(&self) -> Self {
        Self {
            client: self.client.clone(),
            search_terms: self.search_terms.clone(),
        }
    }
}
