//! Module for fetching similar artists from last.fm API. The implementation is
//! kind of weird for now because `Artist`s are really only a means to end
//! (`ArtistTree`).

use std::f64;

use actix_web::http::StatusCode;
use actix_web::ResponseError;
use indexmap::IndexMap;
use serde::de;
use serde::Deserialize;
use serde::Deserializer;
use serde_json::Value;

use crate::get_api_key;
use crate::get_json;
use crate::utils::build_lastfm_url;
use crate::SqPool;
use crate::LASTFM_KEY;

#[derive(Deserialize, Debug, Clone)]
pub struct Artist {
    pub name: String,
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
    #[error("Not found {0}")]
    // ParseError(String),
    // #[error("Not found")]
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

/// A convenience struct used when iterating over a json array
#[derive(Deserialize, Debug, Clone)]
pub struct SimilarArtist {
    // Always canonical
    pub name: String,

    /// Deserialized as `f64`, but stored in db as `i64` (since sqlite has no
    /// `NUMERIC` type)
    #[serde(rename = "match", deserialize_with = "str_to_f64")]
    pub similarity: f64,
}

impl PartialEq for SimilarArtist {
    fn eq(
        &self,
        other: &Self,
    ) -> bool {
        self.name == other.name
    }
}

impl Artist {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
        }
    }

    /// Important: a Last.fm API key is required
    ///
    /// Fetches from db if `self.name` has been cached in the `artists` table.
    /// Otherwise, a network request to last.fm is made, and the request is
    /// processed and cached so it can be skipped the next time.
    ///
    /// Notes:
    /// - `self.name` will **not** be included in the map's keys
    /// - the maximum similarity is 100
    /// - sort order is similarity, descending (insertion order is preserved)
    /// - currently this requires `&mut self`, which is unintuitive; this should
    ///   be changed in future
    pub async fn get_similar_artists(
        &self,
        pool: &SqPool,
        // ) -> anyhow::Result<IndexMap<String, i64>> {
    ) -> Result<IndexMap<String, i64>, LastfmError> {
        if self.canonical_name(pool).await?.is_some() {
            if let Ok(Some(cached)) = self.get_cached_similar_artists(pool).await {
                return Ok(cached);
            }
        }

        let key = get_api_key(pool).await?.ok_or(LastfmError::NoApiKey)?;

        let url = build_lastfm_url("artist.getsimilar", &key, &[("artist", &self.name)]).unwrap();

        // TODO: NotFound variant is somewhere here...

        // String -> Value -> struct
        let resp = reqwest::get(url).await?.text().await?;
        let json: Value = serde_json::from_str(&resp)?;
        let json = &json["similarartists"];

        let canon_name: String = serde_json::from_value(json["@attr"]["artist"].clone())?;
        self.store(pool, &canon_name).await?;

        // let mut map = HashMap::new(); // HashMap uses arbitrary order
        // let mut map = BTreeMap::new(); // BTreeMap always sorts by key
        let mut map = IndexMap::new();

        let similars: Vec<SimilarArtist> = serde_json::from_value(json["artist"].clone())?;

        for sim in similars {
            // it could have been possible to remove the `parent` arg in store_pair (and
            // call self.canonical_name() instead), but that leads to needless db checks
            self.store_pair(pool, &canon_name, &sim.name, sim.similarity)
                .await?;
            map.insert(sim.name, (sim.similarity * 100.0) as i64);
        }

        Ok(map)
    }

    pub async fn get_listeners(
        &self,
        pool: &SqPool,
    ) -> anyhow::Result<u32> {
        if let Ok(Some(Some(x))) = self.get_listeners_db(pool).await {
            return Ok(x.try_into()?);
        };

        let url = build_lastfm_url("artist.getinfo", &LASTFM_KEY, &[("artist", &self.name)])?;
        let json = get_json(url.as_ref()).await?;

        let listeners: String =
            serde_json::from_value(json["artist"]["stats"]["listeners"].clone())?;
        let listeners: u32 = listeners.parse()?;

        self.store_listeners(pool, listeners).await?;

        Ok(listeners)
    }

    pub async fn get_tags(
        &self,
        pool: &SqPool,
    ) -> anyhow::Result<Vec<String>> {
        // if let Ok(Some(Some(x))) = self.get_with_listeners(pool).await {
        //     return Ok(x.try_into()?);
        // };

        let url = build_lastfm_url("artist.getinfo", &LASTFM_KEY, &[("artist", &self.name)])?;
        let json = get_json(url.as_ref()).await?;

        let tags: Vec<Value> = serde_json::from_value(json["artist"]["tags"]["tag"].clone())?;

        // custom deserializer might help
        let tags = tags
            .iter()
            .map(|tag| serde_json::from_value::<String>(tag["name"].clone()).unwrap())
            .collect();

        self.store_tags(pool, &tags).await?;

        Ok(tags)
    }
}

#[cfg(test)]
mod tests {
    use crate::artists::Artist;
    use crate::get_api_key;
    use crate::tests::TestPool;
    use crate::LASTFM_KEY;

    async fn check_similars(
        parent: &str,
        children: &[&str],
    ) {
        let pool = &TestPool::new(Some(&LASTFM_KEY)).await.pool;
        let artist = Artist::new(parent);

        let retrieved = artist.get_similar_artists(pool).await.unwrap();
        assert_eq!(retrieved.len(), 100);
        assert_eq!(retrieved.values().max(), Some(&100));

        println!("{:#?}", artist.get_artist_pairs(pool).await);
        let stored = artist.get_artist_pairs(pool).await.unwrap().unwrap();
        assert_eq!(stored.len(), 100);
        assert_eq!(
            stored
                .iter()
                .filter(|e| e.similarity >= 70)
                .map(|e| e.child.as_str())
                // note: truncation may not always be desired
                .take(children.len())
                .collect::<Vec<&str>>(),
            children
        );
    }

    #[tokio::test]
    async fn no_key() {
        let pool = &TestPool::new(None).await.pool;
        let artist = Artist::new("loona");

        assert!(get_api_key(pool).await.unwrap().is_none());

        let retrieved = artist.get_similar_artists(pool).await;
        assert!(retrieved.is_err());
    }

    #[tokio::test]
    async fn invalid_key() {
        let pool = &TestPool::new(Some("foo")).await.pool;
        let artist = Artist::new("loona");

        println!("{:#?}", get_api_key(pool).await.unwrap());

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

        // TODO: not the most stable test case; this is mainly for testing how weird
        // chars are returned
        check_similars(
            "sadwrist",
            &[
                "tsujiura",
                "MAZES PURR",
                "Where Swans Will Weep",
                "%%%VVV\\/\\/\\/∆∆∆∂∂∂+†*⤴⤴⤴™√Æı∆Æ|†◊æ~∂æ¬#☀\u{fe0e}☽",
            ],
        )
        .await;

        check_similars(
            "pestilence",
            &["Atheist", "Malevolent Creation", "Nocturnus"],
        )
        .await;
    }

    #[tokio::test]
    async fn cached_result() {
        let pool = &TestPool::new(Some(&LASTFM_KEY)).await.pool;
        let artist = Artist::new("loona");

        // TODO: test that only 1 (?) http request made -- Mock seems unsuitable, since
        // it tests requests made to our (mocked) server. we want to check the
        // number of outgoing GET requests to any server
        artist.get_similar_artists(pool).await.unwrap();
        artist.get_similar_artists(pool).await.unwrap();
    }

    #[tokio::test]
    async fn get_tags() {
        let pool = &TestPool::new(Some(&LASTFM_KEY)).await.pool;
        let artist = Artist::new("loona");
        assert_eq!(
            artist.get_tags(pool).await.unwrap(),
            ["pop", "female vocalists", "dance", "k-pop", "spanish",]
        )
    }
}
