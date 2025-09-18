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
//! The scraper supports multiple configurable search terms and filters products
//! to only include items where the combined brand and title contains any of the
//! configured search terms (case insensitive).
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

/// Web scraper for extracting jacket listings from Marrkt with configurable search terms.
///
/// This struct encapsulates an HTTP client configured specifically for scraping
/// Marrkt's product search pages. It supports multiple search configurations
/// and maintains connection pooling for efficient operation.
///
/// # HTTP Client Configuration
///
/// The scraper uses a `reqwest::Client` configured with:
/// - **User Agent**: macOS Safari to avoid bot detection
/// - **Connection Pooling**: Reuses connections for efficiency
/// - **Timeout Handling**: Built-in request timeout management
///
/// # Search Configuration
///
/// The scraper supports multiple search terms configured via:
/// - **Default Terms**: "n-1 deck jacket" and "deck jacket" for comprehensive coverage
/// - **Configurable Terms**: Can be customized for different search strategies
/// - **Filter Logic**: Products must match at least one configured search term
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
    search_terms: Vec<String>,
}

impl Scraper {
    /// Creates a new scraper instance with default search terms and optimized HTTP client.
    ///
    /// This method initializes a `reqwest::Client` with settings optimized for
    /// scraping Marrkt while being respectful of their servers. It configures
    /// default search terms for comprehensive jacket discovery.
    ///
    /// # Default Search Terms
    ///
    /// The scraper is configured with these search terms by default:
    /// - **"n-1 deck jacket"**: Specific N-1 deck jacket searches
    /// - **"deck jacket"**: Broader deck jacket category searches
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
    /// // Scraper is now ready for search operations with default terms
    /// ```
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

    /// Creates a new scraper instance with custom search terms.
    ///
    /// This method allows configuring custom search terms for specialized
    /// monitoring scenarios. Each search term will be used to query Marrkt
    /// and results will be combined and deduplicated.
    ///
    /// # Parameters
    ///
    /// - `search_terms`: Vector of search terms to use for product discovery
    ///
    /// # Search Term Guidelines
    ///
    /// - **Specificity**: More specific terms yield fewer but more relevant results
    /// - **Broad Terms**: General terms like "jacket" may return too many results
    /// - **Combinations**: Terms like "n-1 deck jacket" work well for targeted searches
    /// - **Case Insensitive**: Search terms are automatically handled case-insensitively
    ///
    /// # Examples
    ///
    /// ```rust
    /// use jacket_finder::scraper::Scraper;
    ///
    /// // Custom search configuration for specific jacket types
    /// let search_terms = vec![
    ///     "n-1 deck jacket".to_string(),
    ///     "deck jacket".to_string(),
    ///     "flight jacket".to_string(),
    ///     "bomber jacket".to_string(),
    /// ];
    ///
    /// let scraper = Scraper::with_search_terms(search_terms);
    /// ```
    #[allow(dead_code)]
    pub fn with_search_terms(search_terms: Vec<String>) -> Self {
        let client = Client::builder()
            .user_agent("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36")
            .build()
            .expect("Failed to create HTTP client");

        Self {
            client,
            search_terms,
        }
    }

    /// Searches Marrkt for jacket listings using all configured search terms.
    ///
    /// This method performs multiple search operations based on the configured search terms:
    /// 1. **Multiple Queries**: Performs separate searches for each search term
    /// 2. **HTML Parsing**: Parses responses using the `scraper` crate
    /// 3. **Data Extraction**: Extracts jacket details using CSS selectors
    /// 4. **Filtering**: Only includes items matching any configured search term
    /// 5. **Deduplication**: Removes duplicate results based on product URLs
    /// 6. **URL Generation**: Creates unique IDs and normalizes URLs
    ///
    /// # Search Strategy
    ///
    /// For each search term, the method:
    /// - URL-encodes the search term for Marrkt's search endpoint
    /// - Performs HTTP request to `https://www.marrkt.com/search?q={encoded_term}`
    /// - Extracts and validates product information
    /// - Combines results from all search terms
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
    /// Products are only included if their combined brand and title contains
    /// any of the configured search terms (case insensitive matching).
    ///
    /// # Deduplication Strategy
    ///
    /// Since multiple search terms may return overlapping results, products
    /// are deduplicated based on their URL to ensure each item appears only once.
    ///
    /// # ID Generation
    ///
    /// Each jacket gets a unique ID generated by taking the MD5 hash of its product URL
    /// and formatting it as a hexadecimal string. This ensures consistent identification
    /// across scraping sessions and search terms.
    ///
    /// # Image URL Handling
    ///
    /// The scraper handles various image URL formats:
    /// - **Lazy Loading**: Checks `data-src` attribute first, then `src`
    /// - **Protocol Relative**: Converts `//cdn.example.com/image.jpg` to `https://...`
    /// - **Relative Paths**: Converts `/path/image.jpg` to `https://www.marrkt.com/path/...`
    /// - **Absolute URLs**: Uses as-is for `https://...` URLs
    /// - **Width Placeholders**: Replaces `{width}` with `800` for consistent Discord display
    ///
    /// # Performance Considerations
    ///
    /// - **Sequential Requests**: Searches are performed sequentially to be respectful
    /// - **Connection Reuse**: HTTP client maintains connection pool across searches
    /// - **Memory Efficiency**: Results are collected and deduplicated in memory
    ///
    /// # Returns
    ///
    /// - `Ok(Vec<Jacket>)`: Successfully extracted and deduplicated jacket listings
    /// - `Err`: Network failure, parsing error, or invalid HTML structure
    ///
    /// # Errors
    ///
    /// This method can fail if:
    /// - Network connection to Marrkt fails for any search term
    /// - Marrkt returns non-200 HTTP status for any request
    /// - HTML structure has changed significantly
    /// - Response is not valid UTF-8
    ///
    /// # Examples
    ///
    /// ```rust
    /// use jacket_finder::scraper::Scraper;
    ///
    /// # async fn example() -> anyhow::Result<()> {
    /// let scraper = Scraper::new(); // Uses default search terms
    ///
    /// match scraper.search_jackets().await {
    ///     Ok(jackets) => {
    ///         println!("Found {} jackets across all search terms", jackets.len());
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
                    let url = if href.starts_with("http") {
                        href.to_string()
                    } else {
                        format!("https://www.marrkt.com{href}")
                    };

                    // Skip if we've already processed this URL
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
            search_terms: self.search_terms.clone(),
        }
    }
}
