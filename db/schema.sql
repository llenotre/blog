CREATE TABLE IF NOT EXISTS analytics (
    date TIMESTAMP NOT NULL,
    peer_addr INET,
    user_agent TEXT,
    geolocation JSON,
    device JSON,
    method TEXT NOT NULL,
    uri TEXT NOT NULL
);
CREATE INDEX date ON analytics(date);
CREATE UNIQUE INDEX raw_info ON analytics(peer_addr, user_agent, method, uri);

CREATE TABLE IF NOT EXISTS newsletter_subscriber (
    email TEXT PRIMARY KEY,
    subscribe_date TIMESTAMP NOT NULL,
    unsubscribe_date TIMESTAMP,
    unsubscribe_token UUID
);

CREATE TABLE IF NOT EXISTS "user" (
    id SERIAL PRIMARY KEY,
    access_token TEXT NOT NULL,
    github_login TEXT NOT NULL,
    github_id BIGINT NOT NULL,
    admin BOOLEAN NOT NULL
);
