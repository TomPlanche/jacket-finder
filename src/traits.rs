//! Traits and interfaces for website-agnostic scraping

use anyhow::Result;
use async_trait::async_trait;

use crate::models::Jacket;

/// Configuration for a website scraper
#[derive(Debug, Clone)]
pub struct ScraperConfig {
    /// Display name for the website
    pub name: String,
    /// Base URL for the website
    pub base_url: String,
    /// Search URL pattern with {query} placeholder
    pub search_url_pattern: String,
    /// CSS selectors for extracting data
    pub selectors: SiteSelectors,
    /// Search terms specific to this website
    pub search_terms: Vec<String>,
}

/// CSS selectors for different parts of a product listing
#[derive(Debug, Clone)]
pub struct SiteSelectors {
    /// Container selector for individual products
    pub product_container: String,
    /// Title/name selector within product container
    pub title: String,
    /// Price selector within product container
    pub price: String,
    /// Brand selector within product container (optional)
    pub brand: Option<String>,
    /// Product link selector within product container
    pub link: String,
    /// Image selector within product container
    pub image: String,
    /// Pagination container selector
    pub pagination_container: String,
    /// Next page link selector within pagination
    pub pagination_next: String,
    /// Sold out indicator selector (optional)
    pub sold_out_indicator: Option<String>,
}

/// Trait for website-specific scrapers
#[async_trait]
pub trait WebsiteScraper: Send + Sync {
    /// Get the configuration for this scraper
    fn config(&self) -> &ScraperConfig;
    
    /// Search for jackets on this website
    /// 
    /// # Returns
    /// * `Result<Vec<Jacket>>` - Vector of found jackets or scraping error
    async fn search_jackets(&self) -> Result<Vec<Jacket>>;
    
    /// Extract the next page URL from pagination HTML
    /// 
    /// # Arguments
    /// * `document` - The parsed HTML document
    /// 
    /// # Returns
    /// * `Option<String>` - The next page URL if found
    fn extract_next_page_url(&self, document: &scraper::Html) -> Option<String>;
    
    /// Process a search term to create the search URL
    /// 
    /// # Arguments
    /// * `search_term` - The term to search for
    /// 
    /// # Returns
    /// * `String` - The complete search URL
    fn build_search_url(&self, search_term: &str) -> String {
        let encoded_term = urlencoding::encode(search_term);
        self.config().search_url_pattern.replace("{query}", &encoded_term)
    }
    
    /// Get the user agent string for HTTP requests
    #[allow(dead_code)]
    fn user_agent(&self) -> &'static str {
        "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36"
    }
}
