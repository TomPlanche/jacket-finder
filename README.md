# N-1 Deck Jacket Finder Bot

A Rust bot that monitors [Marrkt](https://www.marrkt.com/search) every 5 minutes for new N-1 deck jacket listings and sends Discord notifications when new items are found.

## Features

- 🔍 Searches Marrkt for "n-1 deck jacket" listings every 5 minutes
- 💾 Stores found jackets in a SQLite database to avoid duplicate notifications
- 🚀 Sends rich Discord notifications with jacket details, price, and direct links
- 📊 Logging with detailed information about bot activity

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

1. **Initial scan:** The bot runs immediately on startup to find existing jackets
2. **Scheduled monitoring:** Every 5 minutes, it searches Marrkt for new listings
3. **Duplicate detection:** Uses a SQLite database to track previously seen jackets
4. **Notifications:** Sends Discord messages only for genuinely new jackets

## Project Structure

```
src/
├── main.rs              # Application entry point and scheduler
├── jacket_finder.rs     # Main coordination logic
├── models/              # Data structures and types
├── database/            # Database operations
├── scraper/             # Web scraping functionality
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

## Customization

The scraper is configured for Marrkt's current HTML structure:
- **Product containers:** `.product-card-wrapper`
- **Titles:** `.product-title a, .card-title a` 
- **Brands:** `.card-subtitle`
- **Prices:** `.product-price-exc-vat`
- **Images:** `.responsive-image__image` with lazy loading support

To modify the search criteria:
- Change the search URL query parameters in `src/scraper/mod.rs`
- Adjust the filtering logic for titles (currently matches "n-1" or "deck jacket")
- Update selectors if Marrkt changes their HTML structure

## Troubleshooting

- **No jackets found:** The HTML selectors may need adjustment if Marrkt updates their site
- **Discord not working:** Verify your webhook URL is correct and the bot has internet access
- **Database errors:** Ensure the directory is writable for SQLite database creation