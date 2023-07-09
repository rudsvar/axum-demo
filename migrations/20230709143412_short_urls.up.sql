CREATE TABLE short_urls (
    id SERIAL PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    target TEXT NOT NULL,
    created_by INTEGER NOT NULL REFERENCES users(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
