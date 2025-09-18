//! # Database Operations
//!
//! This module provides persistent storage for jacket listings using `SQLite` with `SQLx`.
//! It handles database creation, schema migrations, and CRUD operations for jacket data.
//!
//! ## Features
//!
//! - **Automatic Setup**: Creates database and tables on first run
//! - **Schema Migrations**: Uses `SQLx` migrations for versioned schema changes
//! - **Duplicate Prevention**: Tracks seen jackets by unique ID to prevent duplicate notifications
//! - **Async Operations**: All database operations are fully async for better performance
//! - **Error Handling**: Comprehensive error handling with meaningful error messages
//!
//! ## Database Schema
//!
//! The `jackets` table structure:
//! ```sql
//! CREATE TABLE jackets (
//!     id TEXT PRIMARY KEY,           -- MD5 hash of the product URL
//!     title TEXT NOT NULL,           -- Brand + product name
//!     price TEXT NOT NULL,           -- Price as shown on Marrkt
//!     url TEXT NOT NULL,             -- Direct link to product page
//!     image_url TEXT,                -- Optional product image URL
//!     discovered_at DATETIME NOT NULL -- UTC timestamp when first found
//! );
//! ```
//!
//! ## File Location
//!
//! The database file is created at `database/jackets.db` relative to the project root.

use anyhow::Result;
use sqlx::{Row, Sqlite, SqlitePool, migrate::MigrateDatabase};
use std::collections::HashSet;
use tracing::info;

use crate::models::Jacket;

/// Database connection handler for jacket persistence operations.
///
/// This struct encapsulates a `SQLite` connection pool and provides high-level
/// methods for working with jacket data. It handles all database setup,
/// migrations, and CRUD operations.
///
/// # Thread Safety
///
/// `Database` is designed to be cloned and shared across async tasks.
/// The underlying `SqlitePool` handles connection management and thread safety.
///
/// # Examples
///
/// ```rust
/// use jacket_finder::database::Database;
///
/// # async fn example() -> anyhow::Result<()> {
/// // Create database with automatic setup
/// let db = Database::new().await?;
///
/// // Use across multiple tasks
/// let db_clone = db.clone();
/// tokio::spawn(async move {
///     let existing = db_clone.get_existing_jacket_ids().await.unwrap();
///     println!("Found {} existing jackets", existing.len());
/// });
/// # Ok(())
/// # }
/// ```
pub struct Database {
    pool: SqlitePool,
}

impl Database {
    /// Creates a new database connection with automatic setup.
    ///
    /// This method handles the complete database initialization process:
    /// 1. **Database Creation**: Creates the `SQLite` file if it doesn't exist
    /// 2. **Connection Pool**: Establishes a connection pool for concurrent access
    /// 3. **Schema Migrations**: Runs all pending migrations from `./migrations/`
    /// 4. **Validation**: Ensures the database is ready for operations
    ///
    /// The database file is created at `database/jackets.db` relative to the
    /// project root. The `database/` directory must exist or be writable.
    ///
    /// # Returns
    ///
    /// - `Ok(Database)`: Successfully initialized database connection
    /// - `Err`: Database file creation, connection, or migration failure
    ///
    /// # Errors
    ///
    /// This method can fail if:
    /// - The `database/` directory is not writable
    /// - `SQLite` connection cannot be established
    /// - Migration files are corrupted or contain invalid SQL
    /// - Database file permissions are insufficient
    ///
    /// # Examples
    ///
    /// ```rust
    /// use jacket_finder::database::Database;
    ///
    /// # async fn example() -> anyhow::Result<()> {
    /// match Database::new().await {
    ///     Ok(db) => println!("Database ready for operations"),
    ///     Err(e) => eprintln!("Database setup failed: {}", e),
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn new() -> Result<Self> {
        let db_url = "sqlite:database/jackets.db";

        // Create database file if it doesn't exist
        if !Sqlite::database_exists(db_url).await.unwrap_or(false) {
            info!("Creating database file");
            Sqlite::create_database(db_url).await?;
        }

        let pool = SqlitePool::connect(db_url).await?;

        // Run migrations
        info!("Running database migrations");
        sqlx::migrate!("./migrations").run(&pool).await?;

        info!("Database initialized successfully");
        Ok(Self { pool })
    }

    /// Retrieves all existing jacket IDs from the database.
    ///
    /// This method queries the database for all jacket IDs currently stored,
    /// returning them as a `HashSet` for efficient duplicate checking.
    /// It's primarily used to determine which jackets are new versus already seen.
    ///
    /// # Returns
    ///
    /// - `Ok(HashSet<String>)`: Set of all existing jacket IDs
    /// - `Err`: Database query failure
    ///
    /// # Performance
    ///
    /// - Uses `SELECT id FROM jackets` for minimal data transfer
    /// - Returns a `HashSet` for O(1) lookup performance
    /// - Suitable for frequent duplicate checking operations
    ///
    /// # Examples
    ///
    /// ```rust
    /// use jacket_finder::database::Database;
    ///
    /// # async fn example() -> anyhow::Result<()> {
    /// let db = Database::new().await?;
    /// let existing_ids = db.get_existing_jacket_ids().await?;
    ///
    /// if existing_ids.contains("jacket_id_123") {
    ///     println!("Jacket already exists, skipping notification");
    /// } else {
    ///     println!("New jacket found!");
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get_existing_jacket_ids(&self) -> Result<HashSet<String>> {
        let rows = sqlx::query("SELECT id FROM jackets")
            .fetch_all(&self.pool)
            .await?;

        let ids = rows
            .into_iter()
            .map(|row| row.get::<String, _>("id"))
            .collect();

        Ok(ids)
    }

    /// Saves a new jacket to the database.
    ///
    /// This method inserts a complete jacket record into the database,
    /// storing all jacket metadata for future reference and duplicate prevention.
    /// The jacket's unique ID should be generated before calling this method.
    ///
    /// # Parameters
    ///
    /// - `jacket`: Reference to the jacket to save. All fields will be stored.
    ///
    /// # Returns
    ///
    /// - `Ok(())`: Jacket successfully saved to database
    /// - `Err`: Database insertion failure (e.g., constraint violation, connection error)
    ///
    /// # Database Constraints
    ///
    /// - **Primary Key**: The `jacket.id` must be unique
    /// - **NOT NULL**: `title`, `price`, `url`, and `discovered_at` are required
    /// - **Optional**: `image_url` can be `None`
    ///
    /// # Errors
    ///
    /// This method can fail if:
    /// - A jacket with the same ID already exists (primary key constraint)
    /// - Required fields contain `NULL` values
    /// - Database connection is lost
    /// - Database is read-only or full
    ///
    /// # Examples
    ///
    /// ```rust
    /// use jacket_finder::{database::Database, models::Jacket};
    /// use chrono::Utc;
    ///
    /// # async fn example() -> anyhow::Result<()> {
    /// let db = Database::new().await?;
    ///
    /// let jacket = Jacket {
    ///     id: "unique_id_123".to_string(),
    ///     title: "Mister Freedom - N-1 Deck Jacket".to_string(),
    ///     price: "€349,95".to_string(),
    ///     url: "https://www.marrkt.com/products/jacket".to_string(),
    ///     image_url: Some("https://cdn.marrkt.com/image.jpg".to_string()),
    ///     discovered_at: Utc::now(),
    /// };
    ///
    /// match db.save_jacket(&jacket).await {
    ///     Ok(()) => println!("Jacket saved successfully"),
    ///     Err(e) => eprintln!("Failed to save jacket: {}", e),
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn save_jacket(&self, jacket: &Jacket) -> Result<()> {
        sqlx::query(
            r"
            INSERT INTO jackets (id, title, price, url, image_url, discovered_at)
            VALUES (?, ?, ?, ?, ?, ?)
            ",
        )
        .bind(&jacket.id)
        .bind(&jacket.title)
        .bind(&jacket.price)
        .bind(&jacket.url)
        .bind(&jacket.image_url)
        .bind(jacket.discovered_at)
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}

/// Clone implementation for Database to support shared access across async tasks.
///
/// Cloning a `Database` instance creates a new handle to the same underlying
/// connection pool. This is efficient and safe - the `SQLite` connection pool
/// handles concurrent access and connection management internally.
///
/// # Performance
///
/// Cloning is cheap (O(1)) as it only increments reference counters for the
/// underlying connection pool. No new database connections are created.
///
/// # Thread Safety
///
/// Multiple cloned `Database` instances can be safely used across different
/// async tasks and threads simultaneously.
impl Clone for Database {
    fn clone(&self) -> Self {
        Self {
            pool: self.pool.clone(),
        }
    }
}
