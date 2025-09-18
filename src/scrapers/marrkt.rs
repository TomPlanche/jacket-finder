//! Marrkt.com specific scraper implementation

use anyhow::Result;
use async_trait::async_trait;
use chrono::Utc;
use reqwest::Client;
use scraper::{Html, Selector};
use tracing::info;

use crate::models::Jacket;
use crate::traits::{ScraperConfig, SiteSelectors, WebsiteScraper};

/// Scraper implementation for Marrkt.com
pub struct MarrktScraper {
    client: Client,
    config: ScraperConfig,
}

impl MarrktScraper {
    /// Create a new Marrkt scraper with default configuration
    pub fn new() -> Result<Self> {
        let client = Client::builder()
            .user_agent("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36")
            .build()?;

        let config = ScraperConfig {
            name: "Marrkt".to_string(),
            base_url: "https://www.marrkt.com".to_string(),
            search_url_pattern: "https://www.marrkt.com/search?q={query}".to_string(),
            selectors: SiteSelectors {
                product_container: ".product-card-wrapper".to_string(),
                title: ".product-title a, .card-title a".to_string(),
                price: ".product-price-exc-vat".to_string(),
                brand: Some(".card-subtitle".to_string()),
                link: ".product-card a, .card-image a".to_string(),
                image: ".responsive-image__image".to_string(),
                pagination_container: "ul.pagination".to_string(),
                pagination_next: "a.pagination-next".to_string(),
                sold_out_indicator: Some(".card-body p".to_string()),
            },
            search_terms: vec!["n-1 deck jacket".to_string(), "deck jacket".to_string()],
        };

        Ok(Self { client, config })
    }
}

#[async_trait]
impl WebsiteScraper for MarrktScraper {
    fn config(&self) -> &ScraperConfig {
        &self.config
    }

    #[allow(clippy::too_many_lines)]
    async fn search_jackets(&self) -> Result<Vec<Jacket>> {
        const MAX_PAGES: u32 = 50; // Safety limit to prevent infinite loops

        info!(
            "Searching for jackets on {} with {} search terms",
            self.config.name,
            self.config.search_terms.len()
        );

        let mut all_jackets = std::collections::HashMap::new(); // For deduplication by URL

        // Parse selectors with proper error handling
        let product_selector = Selector::parse(&self.config.selectors.product_container)
            .map_err(|e| anyhow::anyhow!("Failed to parse product selector: {:?}", e))?;
        let title_selector = Selector::parse(&self.config.selectors.title)
            .map_err(|e| anyhow::anyhow!("Failed to parse title selector: {:?}", e))?;
        let price_selector = Selector::parse(&self.config.selectors.price)
            .map_err(|e| anyhow::anyhow!("Failed to parse price selector: {:?}", e))?;
        let link_selector = Selector::parse(&self.config.selectors.link)
            .map_err(|e| anyhow::anyhow!("Failed to parse link selector: {:?}", e))?;
        let image_selector = Selector::parse(&self.config.selectors.image)
            .map_err(|e| anyhow::anyhow!("Failed to parse image selector: {:?}", e))?;

        // Optional selectors
        let brand_selector = self.config.selectors.brand.as_ref()
            .map(|s| Selector::parse(s))
            .transpose()
            .map_err(|e| anyhow::anyhow!("Failed to parse brand selector: {:?}", e))?;
        let sold_out_selector = self.config.selectors.sold_out_indicator.as_ref()
            .map(|s| Selector::parse(s))
            .transpose()
            .map_err(|e| anyhow::anyhow!("Failed to parse sold out selector: {:?}", e))?;

        for search_term in &self.config.search_terms {
            info!("Searching for: {} on {}", search_term, self.config.name);

            let mut current_url = self.build_search_url(search_term);
            let mut page_num = 1;

            // Follow pagination until no more pages
            loop {
                if page_num > MAX_PAGES {
                    info!("Reached maximum page limit ({}) for search term: {} on {}",
                          MAX_PAGES, search_term, self.config.name);
                    break;
                }

                info!("Fetching page {} for search term: {} on {}", page_num, search_term, self.config.name);

                let response = self.client.get(&current_url).send().await?;

                if !response.status().is_success() {
                    return Err(anyhow::anyhow!(
                        "Failed to fetch search page {} for '{}' on {}: {}",
                        page_num,
                        search_term,
                        self.config.name,
                        response.status()
                    ));
                }

                let html = response.text().await?;

                // Process the page in a scope to ensure document is dropped before await
                let next_page_url = {
                    let document = Html::parse_document(&html);

                    // Extract next page URL first
                    let next_page_url = self.extract_next_page_url(&document);

                    for product in document.select(&product_selector) {
                        if let Some(link) = product.select(&link_selector).next()
                            && let Some(href) = link.value().attr("href")
                        {
                            let mut url = if href.starts_with("http") {
                                href.to_string()
                            } else {
                                format!("{}{}", self.config.base_url, href)
                            };

                            // Normalize URL by removing query parameters to avoid duplicates
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

                            let brand = if let Some(ref brand_sel) = brand_selector {
                                product.select(brand_sel).next().map_or_else(
                                    || "Unknown Brand".to_string(),
                                    |el| el.text().collect::<String>().trim().to_string(),
                                )
                            } else {
                                "Unknown Brand".to_string()
                            };

                            // Combine brand and title for full item name
                            let title = if brand == "Unknown Brand" {
                                product_title
                            } else {
                                format!("{brand} - {product_title}")
                            };

                            // Check if this item matches any of our search terms
                            let title_lower = title.to_lowercase();
                            let matches_search_term = self.config.search_terms
                                .iter()
                                .any(|term| title_lower.contains(&term.to_lowercase()));

                            if !matches_search_term {
                                continue;
                            }

                            // Skip sold out items if we have a selector for them
                            if let Some(ref sold_out_sel) = sold_out_selector {
                                let is_sold_out = product
                                    .select(sold_out_sel)
                                    .any(|el| el.text().collect::<String>().trim() == "Sold Out");

                                if is_sold_out {
                                    continue;
                                }
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
                                        format!("{}{}", self.config.base_url, src)
                                    };

                                    // Replace {width} placeholder with fixed width for Discord display
                                    if processed_url.contains("{width}") {
                                        processed_url = processed_url.replace("{width}", "800");
                                    }

                                    processed_url
                                });

                            // Generate a unique ID based on URL and website
                            let id = format!("{:x}", md5::compute(format!("{}:{}", self.config.name, url)));

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

                    next_page_url
                }; // document is dropped here

                // Check for next page
                if let Some(next_url) = next_page_url {
                    // Validate the next URL to prevent infinite loops on malformed pagination
                    if next_url == current_url {
                        info!("Next page URL is the same as current URL, stopping pagination for: {} on {}",
                              search_term, self.config.name);
                        break;
                    }

                    current_url = next_url;
                    page_num += 1;

                    // Add small delay between pages to be respectful to the server
                    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                } else {
                    info!("No more pages found for search term: {} on {} (searched {} pages)",
                          search_term, self.config.name, page_num);
                    break;
                }
            }
        }

        let jackets: Vec<Jacket> = all_jackets.into_values().collect();
        info!(
            "Found {} unique jackets on {} across all search terms",
            jackets.len(),
            self.config.name
        );
        Ok(jackets)
    }

    fn extract_next_page_url(&self, document: &Html) -> Option<String> {
        let pagination_selector = Selector::parse(&self.config.selectors.pagination_container).ok()?;
        let next_link_selector = Selector::parse(&self.config.selectors.pagination_next).ok()?;

        let pagination = document.select(&pagination_selector).next()?;
        let next_link = pagination.select(&next_link_selector).next()?;
        let href = next_link.value().attr("href")?;

        // Convert relative URL to absolute URL
        if href.starts_with("http") {
            Some(href.to_string())
        } else {
            Some(format!("{}{}", self.config.base_url, href))
        }
    }
}

impl Clone for MarrktScraper {
    fn clone(&self) -> Self {
        Self {
            client: self.client.clone(),
            config: self.config.clone(),
        }
    }
}
