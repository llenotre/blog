CREATE TABLE IF NOT EXISTS article (
    id INT PRIMARY KEY NOT NULL,
    post_date TIMESTAMP NOT NULL,
    content_id INT NOT NULL,
);

CREATE TABLE IF NOT EXISTS article_content (
    id INT PRIMARY KEY NOT NULL,
    edit_date TIMESTAMP NOT NULL,
    title TEXT NOT NULL,
    desc TEXT NOT NULL,
    cover_url TEXT NOT NULL,
    content TEXT NOT NULL,
    tags TEXT NOT NULL,
    public BOOLEAN NOT NULL,
    sponsors BOOLEAN NOT NULL,
    comments_locked NOT NULL,
);

CREATE TABLE IF NOT EXISTS analytics (
    id INT PRIMARY KEY NOT NULL,
    date TIMESTAMP NOT NULL,
    peer_addr INET,
    user_agent TEXT,
    -- TODO geolocation
    -- TODO device
    method: VARCHAR(16),
    uri: VARCHAR(255),
);

CREATE TABLE IF NOT EXISTS newsletter_subscriber (
    id INT PRIMARY KEY NOT NULL,
    email TEXT,
    subscribe_date TIMESTAMP NOT NULL,
);

CREATE TABLE IF NOT EXISTS user (
    id INT PRIMARY KEY NOT NULL,
    access_token VARCHAR(255) NOT NULL, -- TODO (length)
    github_login: VARCHAR(255), -- TODO
    github_id INT, -- TODO
    github_html_url TEXT, -- TODO
    admin BOOLEAN NOT NULL,
    banned BOOLEAN NOT NULL,
    register_date TIMESTAMP NOT NULL,
    last_post TIMESTAMP NOT NULL,
);

CREATE TABLE IF NOT EXISTS comment (
    id INT PRIMARY KEY NOT NULL,
    article_id INT NOT NULL,
    reply_to INT,
    author INT NOT NULL,
    post_date TIMESTAMP NOT NULL,
    content_id INT NOT NULL,
    removed BOOLEAN NOT NULL,
);

CREATE TABLE IF NOT EXISTS comment_content (
    id INT PRIMARY KEY NOT NULL,
    comment_id INT NOT NULL,
    edit_date TIMESTAMP NOT NULL,
    content TEXT NOT NULL,
);
