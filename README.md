# Multi-Website Jacket Finder Bot

A Rust bot that monitors multiple websites for jacket listings and sends Discord notifications when new items are found. The bot features a website-agnostic architecture that makes it easy to add support for new websites.

## Features

- 🌐 **Website-agnostic**: Easy to add support for multiple websites
- 🔍 **Pagination support**: Searches through all pages of results, not just the first page
- 💾 **Duplicate prevention**: SQLite database prevents duplicate notifications
- 🚀 **Rich notifications**: Discord embeds with jacket details, prices, and direct links
- 📊 **Comprehensive logging**: Detailed information about bot activity across all websites
- ⚡ **Concurrent scraping**: Searches multiple websites simultaneously
- 🛡️ **Error isolation**: Issues with one website don't affect others

## Setup

1. **Clone and build:**
   ```bash
   git clone <your-repo>
   cd jacket-finder
   cargo build --release
   ```

2. **Quality checks:**
   ```bash
   # Run strict clippy checks (recommended before commits)
   ./scripts/check.sh
   
   # Or run manually
   cargo clippy --workspace --release --all-targets --all-features -- --deny warnings -D warnings -W clippy::correctness -W clippy::suspicious -W clippy::complexity -W clippy::perf -W clippy::style -W clippy::pedantic
   ```

3. **Set up Discord webhook:**
   - Go to your Discord server settings
   - Navigate to Integrations → Webhooks
   - Create a new webhook and copy the URL
   - Create a `.env` file (copy from `.env.example`):
   ```bash
   cp .env.example .env
   # Edit .env and add your Discord webhook URL
   ```

4. **Run the bot:**
   ```bash
   # Build and run release version
   cargo build --release
   ./target/release/jacket-finder
   
   # Or run in development mode
   cargo run
   ```

## How it works

1. **Initial scan:** The bot runs immediately on startup to find existing jackets across all configured websites
2. **Scheduled monitoring:** Every 5 minutes, it searches all configured websites for new listings
3. **Pagination crawling:** Follows pagination links to search through all pages of results
4. **Duplicate detection:** Uses a SQLite database to track previously seen jackets across all websites
5. **Error isolation:** If one website fails, others continue working normally
6. **Notifications:** Sends Discord messages only for genuinely new jackets

## Supported Websites

Currently supported websites:
- **Marrkt.com**: Searches for N-1 deck jackets and general deck jackets

Adding new websites is straightforward - see the [Adding New Websites](#adding-new-websites) section below.

## Project Structure

```
src/
├── main.rs              # Application entry point and scheduler
├── jacket_finder.rs     # Main coordination logic (manages multiple scrapers)
├── traits.rs            # WebsiteScraper trait and configuration types
├── models/              # Data structures and types
├── database/            # Database operations
├── scrapers/            # Website-specific scraper implementations
│   ├── mod.rs           # Scraper module exports
│   └── marrkt.rs        # Marrkt.com scraper implementation
└── discord/             # Discord notification handling
migrations/
└── 001_create_jackets_table.sql  # Database schema migrations
database/
└── jackets.db           # SQLite database (created automatically)
```

## Database

The bot uses SQLx migrations to manage database schema changes. On first run, it:
1. Creates the `database/jackets.db` SQLite file 
2. Runs all pending migrations from the `migrations/` folder

The database stores:
- Unique jacket IDs (based on URL hash)
- Title, price, URL, and image URL  
- Discovery timestamp

**Adding new migrations:** Create new `.sql` files in `migrations/` with incremental names (e.g., `002_add_new_column.sql`).

## Discord Notifications

Each new jacket triggers a rich embed with:
- 🧥 Jacket title and description  
- 💰 Price information
- 🔗 Direct link to the listing
- 🖼️ Thumbnail image (if available)
- ⏰ Discovery timestamp

## Adding New Websites

The bot's architecture makes it easy to add support for new websites. Here's how:

### 1. Create a New Scraper

Create a new file (e.g., `src/scrapers/yoursite.rs`) using the `MarrktScraper` as a template:

```rust
// Update the ScraperConfig with your website's details
let config = ScraperConfig {
    name: "Your Site".to_string(),
    base_url: "https://yoursite.com".to_string(),
    search_url_pattern: "https://yoursite.com/search?q={query}".to_string(),
    selectors: SiteSelectors {
        product_container: ".product",           // CSS selector for product containers
        title: ".product-title",                // CSS selector for product titles
        price: ".price",                        // CSS selector for prices
        brand: Some(".brand"),                  // Optional: brand selector
        link: ".product-link",                  // CSS selector for product links
        image: ".product-image img",            // CSS selector for images
        pagination_container: ".pagination",    // CSS selector for pagination
        pagination_next: ".next",               // CSS selector for "next page" link
        sold_out_indicator: Some(".sold-out"), // Optional: sold out indicator
    },
    search_terms: vec!["jacket".to_string()],   // Terms to search for
};
```

### 2. Update the Module

Add your scraper to `src/scrapers/mod.rs`:

```rust
pub mod yoursite;
pub use yoursite::YourSiteScraper;
```

### 3. Register the Scraper

Add your scraper to the JacketFinder in `src/jacket_finder.rs`:

```rust
// In the new() method
let yoursite_scraper = YourSiteScraper::new()?;
scrapers.push(Arc::new(yoursite_scraper));
```

### 4. Website-Specific Customizations

Each scraper can customize:
- **Search terms**: What products to look for
- **CSS selectors**: How to extract data from HTML
- **URL patterns**: How to build search URLs
- **Pagination logic**: How to follow next page links
- **Filtering logic**: What products to include/exclude

### 5. Testing

Run the bot and check the logs to see your new website being scraped:

```bash
cargo run
```

## Architecture Overview

The bot uses a trait-based architecture:

- **`WebsiteScraper` trait**: Defines the interface all scrapers must implement
- **`ScraperConfig`**: Contains website-specific configuration (URLs, selectors, etc.)
- **`JacketFinder`**: Orchestrates multiple scrapers and handles notifications
- **Error isolation**: If one website fails, others continue working

This design makes the bot highly extensible while keeping the core logic simple and maintainable.

## Troubleshooting

- **No jackets found:** Check if the HTML selectors need adjustment if websites update their structure
- **Website errors:** Check logs to see which specific website is having issues - others will continue working
- **Discord not working:** Verify your webhook URL is correct and the bot has internet access
- **Database errors:** Ensure the directory is writable for SQLite database creation
- **Scraper not working:** Use browser developer tools to inspect the website's HTML and update CSS selectors

## Contributing

To contribute support for new websites:

1. Fork the repository
2. Add a new scraper following the guide above
3. Test thoroughly with the target website
4. Submit a pull request with your new scraper

Please ensure your scraper:
- Respects the website's robots.txt and terms of service
- Includes appropriate delays between requests
- Handles errors gracefully
- Follows the existing code style and passes all clippy checks