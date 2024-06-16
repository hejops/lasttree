//! Module for fetching similar artists from last.fm API
//!
//! http://ws.audioscrobbler.com/2.0/?method=artist.getsimilar&artist={}&api_key={}&format=json

use std::f64;

use anyhow::Context;
use indexmap::IndexMap;
use serde::de;
use serde::Deserialize;
use serde::Deserializer;
use serde_json::Value;
use urlencoding::encode;

use crate::get_api_key;
use crate::ArtistTree;
use crate::SqPool;

/// Top-level json object returned by last.fm
#[derive(Deserialize, Debug, Clone)]
struct LastfmArtist {
    /// Only used to extract the canonical name (with the correct
    /// capitalisation)
    #[serde(rename = "@attr")]
    attr: Value,

    #[serde(rename = "artist")]
    similar_artists: Vec<Artist>,
}

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
    ) -> anyhow::Result<IndexMap<String, i64>> {
        if self.canonical_name(pool).await?.is_some() {
            let cached = self.get_cached_similar_artists(pool).await?;
            return Ok(cached);
        }

        let key = get_api_key(pool).await?.context("no api key found")?;

        let url = format!(
            "http://ws.audioscrobbler.com/2.0/?method=artist.getsimilar&artist={}&api_key={}&format=json",
            encode(&self.root),
            // *LASTFM_KEY
            key,
    );

        // String -> Value -> struct
        let resp = reqwest::get(url).await?.text().await?;
        let raw_json: Value = serde_json::from_str::<Value>(&resp)?;
        let json = raw_json
            .get("similarartists")
            .context("no similarartists")?;
        let artist: LastfmArtist = serde_json::from_value(json.clone())?;

        let canon: String = serde_json::from_value(
            artist
                .attr
                .get("artist")
                .context("no artist field")?
                .clone(),
        )?;
        // i would have liked to leave mutation of self to callers, but i'd have to
        // return canon_name in addition to map, leading to an ugly function
        // signature
        self.root = canon;
        self.store(pool).await?;

        // let mut map = HashMap::new(); // HashMap uses arbitrary order
        // let mut map = BTreeMap::new(); // BTreeMap always sorts by key
        let mut map = IndexMap::new();

        for child in artist.similar_artists {
            self.store_pair(&child.name, child.similarity, pool).await?;
            map.insert(child.name, (child.similarity * 100.0) as i64);
        }

        Ok(map)
    }
}

#[cfg(test)]
mod tests {
    use crate::get_api_key;
    use crate::tests::TestPool;
    use crate::ArtistTree;

    async fn check_similars(
        parent: &str,
        children: &[&str],
    ) {
        let pool = &TestPool::new().await.with_key().await.pool;
        let mut artist = ArtistTree::new(parent).await.unwrap();

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
        let pool = &TestPool::new().await.pool;
        let mut artist = ArtistTree::new("loona").await.unwrap();

        assert!(get_api_key(pool).await.unwrap().is_none());

        let retrieved = artist.get_similar_artists(pool).await;
        // println!("{:?}", retrieved);
        assert!(retrieved.is_err());
        // panic!();
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
        let pool = &TestPool::new().await.with_key().await.pool;

        let mut artist = ArtistTree::new("loona").await.unwrap();

        // TODO: test that only 1 (?) http request made -- Mock seems unsuitable, since
        // it tests requests made to our (mocked) server. we want to check the
        // number of outgoing GET requests to any server
        artist.get_similar_artists(pool).await.unwrap();
        artist.get_similar_artists(pool).await.unwrap();
    }
}
