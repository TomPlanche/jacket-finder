//! `SQLite` database operations for jacket storage and duplicate prevention

use anyhow::Result;
use sqlx::{Row, Sqlite, SqlitePool, migrate::MigrateDatabase};
use std::collections::HashSet;
use tracing::info;

use crate::models::Jacket;

/// `SQLite` database connection and operations
pub struct Database {
    pool: SqlitePool,
}

impl Database {
    /// Create new database connection with migrations
    /// 
    /// # Returns
    /// * `Result<Self>` - New Database instance or connection/migration error
    pub async fn new() -> Result<Self> {
        let db_url = "sqlite:database/jackets.db";

        if !Sqlite::database_exists(db_url).await.unwrap_or(false) {
            info!("Creating database file");
            Sqlite::create_database(db_url).await?;
        }

        let pool = SqlitePool::connect(db_url).await?;

        info!("Running database migrations");
        sqlx::migrate!("./migrations").run(&pool).await?;

        info!("Database initialized successfully");
        Ok(Self { pool })
    }

    /// Get all existing jacket IDs for duplicate checking
    /// 
    /// # Returns
    /// * `Result<HashSet<String>>` - Set of existing jacket IDs or database error
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

    /// Save a new jacket to the database
    /// 
    /// # Arguments
    /// * `jacket` - Reference to the jacket to save
    /// 
    /// # Returns
    /// * `Result<()>` - Success or database insertion error
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

impl Clone for Database {
    fn clone(&self) -> Self {
        Self {
            pool: self.pool.clone(),
        }
    }
}
