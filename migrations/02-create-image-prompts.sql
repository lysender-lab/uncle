CREATE TABLE image_prompts (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    prompt TEXT NOT NULL,
    short_title TEXT NOT NULL,
    model TEXT NOT NULL,
    background TEXT NOT NULL,
    moderation TEXT NOT NULL,
    qty INTEGER NOT NULL,
    output_compression INTEGER NOT NULL,
    output_format TEXT NOT NULL,
    quality TEXT NOT NULL,
    status TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
) STRICT;

CREATE INDEX idx_image_prompts_user_id ON image_prompts(user_id);
