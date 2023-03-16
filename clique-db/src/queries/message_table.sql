CREATE TABLE IF NOT EXISTS messages (
    id BIGINT PRIMARY KEY REFERENCES users(id),
    guild BIGINT NOT NULL,
    author BIGINT NOT NULL,
    channel BIGINT NOT NULL,
    reply_to BIGINT REFERENCES users(id),
    timestamp TIMESTAMP NOT NULL
);
