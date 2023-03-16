INSERT INTO users (id, username) VALUES ($1, $2)
ON CONFLICT (id) DO UPDATE SET username = $2;
