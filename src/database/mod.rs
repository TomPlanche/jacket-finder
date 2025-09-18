use anyhow::Result;
use sqlx::{Row, Sqlite, SqlitePool, migrate::MigrateDatabase};
use std::collections::HashSet;
use tracing::info;

use crate::models::Jacket;

pub struct Database {
    pool: SqlitePool,
}

impl Database {
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
