use std::collections::BTreeMap;
use std::f64;

use anyhow::Context;
use indexmap::IndexMap;
use serde::de;
use serde::Deserialize;
use serde::Deserializer;
use serde_json::Value;

use super::LASTFM_KEY;
use crate::get_artist_from_db;
use crate::get_artist_pairs;
use crate::store_artist;
use crate::store_artist_pair;
use crate::SqPool;

/// A convenience struct used when iterating over a json array
#[derive(Deserialize, Debug, Clone)]
struct Artist {
    pub name: String,

    /// Deserialized as `f64`, but stored in db as `i64` (since sqlite has no
    /// `NUMERIC` type)
    #[serde(rename = "match", deserialize_with = "str_to_f64")]
    pub similarity: f64,

    /// Used for `ArtistTree::as_html`
    pub url: String,
}

impl PartialEq for Artist {
    fn eq(
        &self,
        other: &Self,
    ) -> bool {
        self.name == other.name
    }
}

/// Top-level
#[derive(Deserialize, Debug, Clone)]
struct LastfmArtist {
    /// Only used to extract the canonical name (with the correct
    /// capitalisation)
    #[serde(rename = "@attr")]
    attr: Value,

    #[serde(rename = "artist")]
    similar_artists: Vec<Artist>,
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

/// `canon` must be found in `artists` table. This allows the hashmap to be
/// built without making any network requests.
pub async fn get_cached_similar_artists(
    canon: &str,
    pool: &SqPool,
) -> anyhow::Result<IndexMap<String, i64>> {
    let mut map = IndexMap::new();
    for pair in get_artist_pairs(canon, pool).await? {
        map.insert(pair.child, pair.similarity);
    }
    println!("using cached result");
    Ok(map)
}

/// Fetches from db if `artist` has been cached in the `artists` table.
/// Otherwise, a network request to last.fm is made, and the request is
/// processed and cached so it can be skipped the next time.
///
/// Notes:
/// - `artist` will **not** be included in the map's keys
/// - the maximum similarity is 100
/// - sort order is similarity, descending
pub async fn get_similar_artists(
    artist: &str,
    pool: &SqPool,
) -> anyhow::Result<IndexMap<String, i64>> {
    if let Some(canon) = get_artist_from_db(artist, pool).await? {
        let cached = get_cached_similar_artists(&canon, pool).await?;
        return Ok(cached);
    }

    let url = format!(
            "http://ws.audioscrobbler.com/2.0/?method=artist.getsimilar&artist={}&api_key={}&format=json",
            artist,
            *LASTFM_KEY
        );

    // String -> Value -> struct
    let resp = reqwest::get(url).await?.text().await?;
    let raw_json: Value = serde_json::from_str::<Value>(&resp)?;
    let json = raw_json
        .get("similarartists")
        .context("no similarartists")?;

    let json: LastfmArtist = serde_json::from_value(json.clone())?;
    let canon_name: String =
        serde_json::from_value(json.attr.get("artist").context("no artist field")?.clone())?;
    store_artist(&canon_name, pool).await?;

    // let mut map = HashMap::new(); // HashMap uses arbitrary order
    // let mut map = BTreeMap::new(); // BTreeMap always sorts by key
    let mut map = IndexMap::new();

    for sim in json.similar_artists {
        store_artist_pair(&canon_name, &sim.name, sim.similarity, pool).await?;
        map.insert(sim.name, (sim.similarity * 100.0) as i64);
    }

    // println!("{}", artist);
    // for (k, v) in map.iter().take(10) {
    //     println!("{k} {v}");
    // }

    // println!("{:#?}", map);
    // panic!();

    Ok(map)
}

#[cfg(test)]
mod tests {

    use crate::get_artist_pairs;
    use crate::get_similar_artists;
    use crate::init_test_db;

    #[tokio::test]
    async fn standard() {
        let pool = &init_test_db().await.pool;

        let retrieved = get_similar_artists("loona", pool).await.unwrap();
        assert_eq!(retrieved.len(), 100);
        assert_eq!(retrieved.values().max(), Some(&100));

        let stored = get_artist_pairs("loona", pool).await.unwrap();
        assert_eq!(stored.len(), 100);
        assert_eq!(stored.iter().filter(|e| e.similarity >= 70).count(), 3);
    }

    #[tokio::test]
    async fn special_chars() {
        let pool = &init_test_db().await.pool;

        let retrieved = get_similar_artists("loona 1/3", pool).await.unwrap();
        assert_eq!(retrieved.len(), 100);
        assert_eq!(retrieved.values().max(), Some(&100));

        let stored = get_artist_pairs("loona 1/3", pool).await.unwrap();
        assert_eq!(stored.len(), 100);
        assert_eq!(stored.iter().filter(|e| e.similarity >= 70).count(), 3);
    }

    #[tokio::test]
    async fn cached_result() {
        let pool = &init_test_db().await.pool;
        // TODO: test that only 1 http request made -- Mock?
        get_similar_artists("loona", pool).await.unwrap();
        get_similar_artists("loona", pool).await.unwrap();
    }
}
