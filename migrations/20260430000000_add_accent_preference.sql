CREATE TYPE accent AS ENUM ('coral', 'iris', 'matcha', 'sakura', 'citron');

ALTER TABLE user_settings
    ADD COLUMN accent_preference accent NOT NULL DEFAULT 'coral';
