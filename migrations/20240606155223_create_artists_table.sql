CREATE TABLE IF NOT EXISTS artists(
	-- -- COLLATE NOCASE is worthless if it doesn't handle unicode!
	-- -- custom collation sounds like the cleanest way, but it is not
	-- -- trivial to get working across the app
	-- -- https://shallowdepth.online/posts/2022/01/5-ways-to-implement-case-insensitive-search-in-sqlite-with-full-unicode-support/
	-- name TEXT NOT NULL UNIQUE COLLATE NOCASE,
	name TEXT NOT NULL UNIQUE,
	-- doubles disk usage
	name_lower TEXT NOT NULL UNIQUE,

	-- -- no uuid type in sqlite
	-- id TEXT NOT NULL,
	PRIMARY KEY (name)
);

