CREATE TABLE IF NOT EXISTS article (
    id SERIAL PRIMARY KEY,
    post_date TIMESTAMP,
    content_id INT NOT NULL
);

CREATE TABLE IF NOT EXISTS article_content (
    id SERIAL PRIMARY KEY,
    article_id INT NOT NULL,
    edit_date TIMESTAMP NOT NULL,
    title TEXT NOT NULL,
    description TEXT NOT NULL,
    cover_url TEXT NOT NULL,
    content TEXT NOT NULL,
    tags TEXT NOT NULL,
    public BOOLEAN NOT NULL,
    sponsors BOOLEAN NOT NULL,
    comments_locked BOOLEAN NOT NULL
);

CREATE TABLE IF NOT EXISTS file (
    id SERIAL PRIMARY KEY,
    name TEXT,
    mime_type TEXT NOT NULL,
    upload_date TIMESTAMP NOT NULL,
    data BYTEA NOT NULL
);

CREATE TABLE IF NOT EXISTS analytics (
    id SERIAL PRIMARY KEY,
    date TIMESTAMP NOT NULL,
    peer_addr INET NOT NULL,
    user_agent TEXT NOT NULL,
    geolocation JSON,
    device JSON,
    method VARCHAR(16) NOT NULL,
    uri VARCHAR(255) NOT NULL
);

CREATE TABLE IF NOT EXISTS newsletter_subscriber (
    id SERIAL PRIMARY KEY,
    email TEXT,
    subscribe_date TIMESTAMP NOT NULL
);

CREATE TABLE IF NOT EXISTS "user" (
    id SERIAL PRIMARY KEY,
    access_token TEXT NOT NULL,
    github_login TEXT NOT NULL,
    github_id BIGINT NOT NULL,
    github_html_url TEXT NOT NULL,
    admin BOOLEAN NOT NULL,
    banned BOOLEAN NOT NULL,
    register_date TIMESTAMP NOT NULL,
    last_post TIMESTAMP NOT NULL
);

CREATE TABLE IF NOT EXISTS comment (
    id SERIAL PRIMARY KEY,
    article_id INT NOT NULL,
    reply_to INT,
    author_id INT NOT NULL,
    post_date TIMESTAMP NOT NULL,
    content_id INT NOT NULL,
    removed TIMESTAMP
);

CREATE TABLE IF NOT EXISTS comment_content (
    id SERIAL PRIMARY KEY,
    edit_date TIMESTAMP NOT NULL,
    content TEXT NOT NULL
);
