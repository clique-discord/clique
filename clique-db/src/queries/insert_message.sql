INSERT INTO messages (
    id, guild, author, channel, reply_to, timestamp
) VALUES ($1, $2, $3, $4, $5, $6);
