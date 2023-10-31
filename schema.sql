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
    sponsor BOOLEAN NOT NULL,
    comments_locked BOOLEAN NOT NULL
);

CREATE TABLE IF NOT EXISTS analytics (
    date TIMESTAMP NOT NULL,
    peer_addr INET NOT NULL,
    user_agent TEXT NOT NULL,
    geolocation JSON,
    device JSON,
    method TEXT NOT NULL,
    uri TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS newsletter_subscriber (
    email TEXT PRIMARY KEY,
    subscribe_date TIMESTAMP NOT NULL,
    unsubscribe_date TIMESTAMP,
    unsubscribe_token UUID
);

CREATE TABLE IF NOT EXISTS newsletter_group (
    id SERIAL PRIMARY KEY NOT NULL,
    subject TEXT NOT NULL,
    content_html TEXT NOT NULL,
    content_text TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS newsletter_email (
    token UUID PRIMARY KEY NOT NULL,
    email_group INT,
    recipient TEXT NOT NULL,
    sent_at TIMESTAMP,
    send_error TEXT
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
    remove_date TIMESTAMP
);

CREATE TABLE IF NOT EXISTS comment_content (
    id SERIAL PRIMARY KEY,
    comment_id INT NOT NULL,
    edit_date TIMESTAMP NOT NULL,
    content TEXT NOT NULL
);
