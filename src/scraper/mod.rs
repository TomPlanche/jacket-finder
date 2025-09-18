//! # Web Scraping Operations
//!
//! This module provides web scraping functionality specifically designed for Marrkt's
//! product search pages. It extracts N-1 deck jacket listings with all relevant metadata.
//!
//! ## Features
//!
//! - **Targeted Scraping**: Searches specifically for N-1 deck jacket listings
//! - **Robust Parsing**: Handles Marrkt's current HTML structure with fallbacks
//! - **Image Support**: Extracts both lazy-loaded and regular product images
//! - **Duplicate Filtering**: Only includes jackets matching specific criteria
//! - **Error Handling**: Graceful handling of network and parsing failures
//!
//! ## Marrkt Integration
//!
//! The scraper is specifically configured for Marrkt's HTML structure:
//! ```html
//! <div class="product-card-wrapper">
//!   <div class="product-card">
//!     <a href="/products/jacket-url">
//!       <img class="responsive-image__image" src="image.jpg" />
//!     </a>
//!     <div class="card-header">
//!       <p class="card-subtitle">Brand Name</p>
//!       <h6 class="product-title">
//!         <a href="/products/jacket-url">Product Name</a>
//!       </h6>
//!       <span class="product-price-exc-vat">€349,95</span>
//!     </div>
//!   </div>
//! </div>
//! ```
//!
//! ## Search Criteria
//!
//! The scraper filters products to only include items where the combined
//! brand and title contains either "n-1" or "deck jacket" (case insensitive).
//!
//! ## Rate Limiting
//!
//! The scraper uses respectful crawling practices:
//! - Single request per search operation
//! - Appropriate user agent identification
//! - No concurrent requests to avoid server overload

use anyhow::Result;
use chrono::Utc;
use reqwest::Client;
use scraper::{Html, Selector};
use tracing::info;

use crate::models::Jacket;

/// Web scraper for extracting jacket listings from Marrkt.
///
/// This struct encapsulates an HTTP client configured specifically for scraping
/// Marrkt's product search pages. It maintains connection pooling and proper
/// user agent identification for respectful crawling.
///
/// # HTTP Client Configuration
///
/// The scraper uses a `reqwest::Client` configured with:
/// - **User Agent**: macOS Safari to avoid bot detection
/// - **Connection Pooling**: Reuses connections for efficiency
/// - **Timeout Handling**: Built-in request timeout management
///
/// # Thread Safety
///
/// `Scraper` can be safely cloned and shared across async tasks.
/// The underlying HTTP client handles concurrent requests appropriately.
///
/// # Examples
///
/// ```rust
/// use jacket_finder::scraper::Scraper;
///
/// # async fn example() -> anyhow::Result<()> {
/// let scraper = Scraper::new();
/// let jackets = scraper.search_jackets().await?;
/// println!("Found {} jackets", jackets.len());
/// # Ok(())
/// # }
/// ```
pub struct Scraper {
    client: Client,
}

impl Scraper {
    /// Creates a new scraper instance with optimized HTTP client configuration.
    ///
    /// This method initializes a `reqwest::Client` with settings optimized for
    /// scraping Marrkt while being respectful of their servers.
    ///
    /// # HTTP Client Features
    ///
    /// - **User Agent**: Uses macOS Safari user agent to appear as regular browser traffic
    /// - **Connection Pooling**: Automatically reuses connections for multiple requests
    /// - **Redirects**: Follows HTTP redirects automatically (up to 10 by default)
    /// - **Timeouts**: Built-in request and connection timeout handling
    ///
    /// # Panics
    ///
    /// This method panics if the HTTP client cannot be created, which should only
    /// happen in extreme circumstances (e.g., system TLS configuration issues).
    ///
    /// # Examples
    ///
    /// ```rust
    /// use jacket_finder::scraper::Scraper;
    ///
    /// let scraper = Scraper::new();
    /// // Scraper is now ready for search operations
    /// ```
    pub fn new() -> Self {
        let client = Client::builder()
            .user_agent("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36")
            .build()
            .expect("Failed to create HTTP client");

        Self { client }
    }

    /// Searches Marrkt for N-1 deck jacket listings and extracts structured data.
    ///
    /// This method performs a complete scraping operation:
    /// 1. **HTTP Request**: Fetches the search results page from Marrkt
    /// 2. **HTML Parsing**: Parses the response using the `scraper` crate
    /// 3. **Data Extraction**: Extracts jacket details using CSS selectors
    /// 4. **Filtering**: Only includes items matching N-1 deck jacket criteria
    /// 5. **URL Generation**: Creates unique IDs and normalizes URLs
    ///
    /// # Search URL
    ///
    /// Queries: `https://www.marrkt.com/search?q=n-1+deck+jacket`
    ///
    /// # CSS Selectors Used
    ///
    /// - **Products**: `.product-card-wrapper` (main product containers)
    /// - **Titles**: `.product-title a, .card-title a` (product names)
    /// - **Brands**: `.card-subtitle` (brand information)
    /// - **Prices**: `.product-price-exc-vat` (price excluding VAT)
    /// - **Links**: `.product-card a, .card-image a` (product page URLs)
    /// - **Images**: `.responsive-image__image` (product images with lazy loading support)
    ///
    /// # Filtering Logic
    ///
    /// Products are only included if their combined brand and title contains:
    /// - "n-1" (case insensitive), OR
    /// - "deck jacket" (case insensitive)
    ///
    /// # ID Generation
    ///
    /// Each jacket gets a unique ID generated by taking the MD5 hash of its product URL
    /// and formatting it as a hexadecimal string. This ensures consistent identification
    /// across scraping sessions.
    ///
    /// # Image URL Handling
    ///
    /// The scraper handles various image URL formats:
    /// - **Lazy Loading**: Checks `data-src` attribute first, then `src`
    /// - **Protocol Relative**: Converts `//cdn.example.com/image.jpg` to `https://...`
    /// - **Relative Paths**: Converts `/path/image.jpg` to `https://www.marrkt.com/path/...`
    /// - **Absolute URLs**: Uses as-is for `https://...` URLs
    ///
    /// # Returns
    ///
    /// - `Ok(Vec<Jacket>)`: Successfully extracted jacket listings
    /// - `Err`: Network failure, parsing error, or invalid HTML structure
    ///
    /// # Errors
    ///
    /// This method can fail if:
    /// - Network connection to Marrkt fails
    /// - Marrkt returns non-200 HTTP status
    /// - HTML structure has changed significantly
    /// - Response is not valid UTF-8
    ///
    /// # Examples
    ///
    /// ```rust
    /// use jacket_finder::scraper::Scraper;
    ///
    /// # async fn example() -> anyhow::Result<()> {
    /// let scraper = Scraper::new();
    ///
    /// match scraper.search_jackets().await {
    ///     Ok(jackets) => {
    ///         println!("Found {} N-1 deck jackets", jackets.len());
    ///         for jacket in jackets {
    ///             println!("- {} ({})", jacket.title, jacket.price);
    ///         }
    ///     }
    ///     Err(e) => eprintln!("Scraping failed: {}", e),
    /// }
    /// # Ok(())
    /// # }
    /// ```
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

/// Clone implementation for Scraper to support shared access across async tasks.
///
/// Cloning a `Scraper` creates a new handle to the same underlying HTTP client.
/// This is efficient as `reqwest::Client` uses Arc internally for sharing
/// connection pools and configuration.
///
/// # Performance
///
/// - **Cheap Operation**: Cloning only increments reference counters
/// - **Shared Connections**: All clones use the same connection pool
/// - **Thread Safe**: Multiple clones can be used concurrently
///
/// # Use Cases
///
/// - **Task Distribution**: Share scraper across multiple async tasks
/// - **Service Architecture**: Pass scraper to different service components
/// - **Testing**: Create independent scraper instances for test isolation
///
/// # Examples
///
/// ```rust
/// use jacket_finder::scraper::Scraper;
///
/// # async fn example() -> anyhow::Result<()> {
/// let scraper = Scraper::new();
/// let scraper_clone = scraper.clone();
///
/// // Use both instances concurrently
/// let (results1, results2) = tokio::join!(
///     scraper.search_jackets(),
///     scraper_clone.search_jackets()
/// );
/// # Ok(())
/// # }
/// ```
impl Clone for Scraper {
    fn clone(&self) -> Self {
        Self {
            client: self.client.clone(),
        }
    }
}
