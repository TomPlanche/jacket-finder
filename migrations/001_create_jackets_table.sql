-- Initial migration: Create jackets table
CREATE TABLE IF NOT EXISTS jackets (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    price TEXT NOT NULL,
    url TEXT NOT NULL,
    image_url TEXT,
    discovered_at DATETIME NOT NULL
);
