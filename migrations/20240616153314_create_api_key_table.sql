CREATE TABLE IF NOT EXISTS api_key(
	-- this table only contains a single column, and a single row
	-- the key is stored in plaintext
	key TEXT NOT NULL UNIQUE,
	PRIMARY KEY (key)
);

