CREATE TABLE blueprints (
    slug TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    properties JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
)