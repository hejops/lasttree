CREATE TABLE IF NOT EXISTS artist_pairs(
	-- by definition of pairs, these str fields cannot be unique!
	parent TEXT NOT NULL,
	-- parent_lower TEXT NOT NULL,
	child TEXT NOT NULL,
	-- child_lower TEXT NOT NULL,
	-- similarity NUMERIC(1,2) NOT NULL,
	similarity INTEGER NOT NULL,

	date_added TEXT NOT NULL,

	-- PRIMARY KEY must always be specified last (else syntax error!)
	-- PRIMARY KEY (parent_lower, child_lower)
	PRIMARY KEY (parent, child)
);

