//! Module for fetching similar artists from last.fm API. Kind of weird because
//! `Artist`s are really only a means to end (`ArtistTree`).
//!
//! http://ws.audioscrobbler.com/2.0/?method=artist.getsimilar&artist={}&api_key={}&format=json

use std::f64;

use actix_web::http::StatusCode;
use actix_web::ResponseError;
use indexmap::IndexMap;
use serde::de;
use serde::Deserialize;
use serde::Deserializer;
use serde_json::Value;
use urlencoding::encode;

use crate::get_api_key;
use crate::ArtistTree;
use crate::SqPool;

/// A convenience struct used when iterating over a json array
#[derive(Deserialize, Debug, Clone)]
struct Artist {
    pub name: String,

    /// Deserialized as `f64`, but stored in db as `i64` (since sqlite has no
    /// `NUMERIC` type)
    #[serde(rename = "match", deserialize_with = "str_to_f64")]
    pub similarity: f64,
}

impl PartialEq for Artist {
    fn eq(
        &self,
        other: &Self,
    ) -> bool {
        self.name == other.name
    }
}

// https://stackoverflow.com/a/75684771
// https://serde.rs/impl-deserialize.html
fn str_to_f64<'de, D: Deserializer<'de>>(deserializer: D) -> Result<f64, D::Error> {
    Ok(match Value::deserialize(deserializer)? {
        Value::String(s) => s.parse().map_err(de::Error::custom)?,
        Value::Number(num) => num
            .as_f64()
            .ok_or_else(|| de::Error::custom("Invalid number"))?,
        _ => return Err(de::Error::custom("wrong type")),
    })
}

// https://github.com/freedomofpress/securedrop/blob/5733557ffa98f03fc9eeb8b3ff763a661ee2875f/redwood/src/lib.rs#L29

// `thiserror::Error` provides `Display` (via `error`), `Error::source` (via
// `source`) and `From` (via `from`). `from` implements -both- `From` and
// `Error::source`
#[derive(thiserror::Error, Debug)]
pub enum LastfmError {
    #[error("No API key")]
    NoApiKey,

    // variant(#[from] module::Error) enables ?
    // but error types must be unique!

    // #[error("Not found {0}")]
    // ParseError(String),
    #[error("Not found")]
    ParseError(#[from] serde_json::Error),

    #[error(transparent)]
    DatabaseError(#[from] sqlx::Error),

    #[error(transparent)]
    NetworkError(#[from] reqwest::Error),
}

impl ResponseError for LastfmError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::NoApiKey => StatusCode::UNAUTHORIZED,
            Self::ParseError(_) => StatusCode::NOT_FOUND, // TODO: is there a better code?
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl ArtistTree {
    /// Important: a Last.fm API key is required
    ///
    /// Fetches from db if `artist` has been cached in the `artists` table.
    /// Otherwise, a network request to last.fm is made, and the request is
    /// processed and cached so it can be skipped the next time.
    ///
    /// Notes:
    /// - `artist` will **not** be included in the map's keys
    /// - the maximum similarity is 100
    /// - sort order is similarity, descending (insertion order is preserved)
    /// - currently this requires `&mut self`, which is unintuitive; this should
    ///   be changed in future
    pub async fn get_similar_artists(
        &mut self,
        pool: &SqPool,
        // ) -> anyhow::Result<IndexMap<String, i64>> {
    ) -> Result<IndexMap<String, i64>, LastfmError> {
        if self.canonical_name(pool).await?.is_some() {
            let cached = self.get_cached_similar_artists(pool).await?;
            return Ok(cached);
        }

        let key = get_api_key(pool).await?.ok_or(LastfmError::NoApiKey)?;

        let url = format!(
            "http://ws.audioscrobbler.com/2.0/?method=artist.getsimilar&artist={}&api_key={}&format=json",
            encode(&self.root),
            // *LASTFM_KEY
            key,
    );

        // TODO: NotFound variant is somewhere here...

        // String -> Value -> struct
        let resp = reqwest::get(url).await?.text().await?;
        // let raw_json: Value = serde_json::from_str::<Value>(&resp)?;
        // let json = &raw_json["similarartists"];

        let json: Value = serde_json::from_str(&resp)?;
        let json = &json["similarartists"];

        // i would have liked to leave mutation of self to callers, but i'd have to
        // return canon_name in addition to map, leading to an ugly function
        // signature
        self.root = serde_json::from_value(json["@attr"]["artist"].clone())?;
        self.store(pool).await?;

        // let mut map = HashMap::new(); // HashMap uses arbitrary order
        // let mut map = BTreeMap::new(); // BTreeMap always sorts by key
        let mut map = IndexMap::new();

        let similars: Vec<Artist> = serde_json::from_value(json["artist"].clone())?;

        for sim in similars {
            self.store_pair(&sim.name, sim.similarity, pool).await?;
            map.insert(sim.name, (sim.similarity * 100.0) as i64);
        }

        Ok(map)
    }
}

#[cfg(test)]
mod tests {
    use crate::get_api_key;
    use crate::tests::TestPool;
    use crate::ArtistTree;
    use crate::LASTFM_KEY;

    async fn check_similars(
        parent: &str,
        children: &[&str],
    ) {
        let pool = &TestPool::new(Some(&LASTFM_KEY)).await.pool;
        let mut artist = ArtistTree::new(parent);

        let retrieved = artist.get_similar_artists(pool).await.unwrap();
        assert_eq!(retrieved.len(), 100);
        assert_eq!(retrieved.values().max(), Some(&100));

        let stored = artist.get_artist_pairs(pool).await.unwrap();
        assert_eq!(stored.len(), 100);
        assert_eq!(
            stored
                .iter()
                .filter(|e| e.similarity >= 70)
                .map(|e| e.child.as_str())
                .collect::<Vec<&str>>(),
            children
        );
    }

    #[tokio::test]
    async fn no_key() {
        let pool = &TestPool::new(None).await.pool;
        let mut artist = ArtistTree::new("loona");

        assert!(get_api_key(pool).await.unwrap().is_none());

        let retrieved = artist.get_similar_artists(pool).await;
        assert!(retrieved.is_err());
    }

    #[tokio::test]
    async fn invalid_key() {
        let pool = &TestPool::new(Some("foo")).await.pool;
        let mut artist = ArtistTree::new("loona");

        assert!(get_api_key(pool).await.unwrap().is_none());

        let retrieved = artist.get_similar_artists(pool).await;
        assert!(retrieved.is_err());
    }

    #[tokio::test]
    async fn get_similar_artists() {
        check_similars(
            "loona",
            &["LOOΠΔ 1/3", "LOONA/yyxy", "LOOΠΔ / ODD EYE CIRCLE"],
        )
        .await;

        check_similars(
            "LOOΠΔ 1/3",
            // note: because "loona 1/3" is considered a different artist, it will produce
            // different children
            &["LOONA/yyxy", "LOOΠΔ / ODD EYE CIRCLE", "Loona"],
        )
        .await;

        check_similars(
            "sadwrist",
            &[
                "tsujiura",
                "Where Swans Will Weep",
                "%%%VVV\\/\\/\\/∆∆∆∂∂∂+†*⤴⤴⤴™√Æı∆Æ|†◊æ~∂æ¬#☀\u{fe0e}☽",
                "MAZES PURR",
            ],
        )
        .await;
    }

    #[tokio::test]
    async fn cached_result() {
        let pool = &TestPool::new(Some(&LASTFM_KEY)).await.pool;

        let mut artist = ArtistTree::new("loona");

        // TODO: test that only 1 (?) http request made -- Mock seems unsuitable, since
        // it tests requests made to our (mocked) server. we want to check the
        // number of outgoing GET requests to any server
        artist.get_similar_artists(pool).await.unwrap();
        artist.get_similar_artists(pool).await.unwrap();
    }
}
