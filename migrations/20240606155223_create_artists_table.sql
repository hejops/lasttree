CREATE TABLE IF NOT EXISTS artists(
	name TEXT NOT NULL UNIQUE,
	name_lower TEXT NOT NULL UNIQUE,
	-- -- no uuid type in sqlite
	-- id TEXT NOT NULL,
	PRIMARY KEY (name_lower)
);

