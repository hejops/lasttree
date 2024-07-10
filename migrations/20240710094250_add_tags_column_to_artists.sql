-- stored via (and deserialised into) Vec<String>
-- requires sqlx `json` feature
ALTER TABLE artists ADD COLUMN tags JSON;

