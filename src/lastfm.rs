use std::collections::HashMap;
use std::f64;

use anyhow::Context;
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
    // pub url: String,
}

/// Top-level
#[derive(Deserialize, Debug, Clone)]
struct LastfmArtist {
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

pub async fn get_lastfm_similar_artists(
    artist: &str,
    pool: &SqPool,
) -> anyhow::Result<HashMap<String, i64>> {
    let mut map = HashMap::new();

    // first check db; if found, build the hashmap without fetching
    if let Some(canon) = get_artist_from_db(artist, pool).await? {
        for pair in get_artist_pairs(&canon, pool).await? {
            map.insert(pair.child, pair.similarity);
        }
        println!("using cached result");
        return Ok(map);
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
    let artist: LastfmArtist = serde_json::from_value(json.clone())?;

    let canon_name: String = serde_json::from_value(
        artist
            .attr
            .get("artist")
            .context("no artist field")?
            .clone(),
    )?;
    store_artist(&canon_name, pool).await?;

    for sim in artist.similar_artists {
        // store_artist(&sim.name, &pool).await?;
        store_artist_pair(&canon_name, &sim.name, sim.similarity, pool).await?;
        map.insert(sim.name, sim.similarity as i64);
    }
    // panic!();

    Ok(map)
}

#[cfg(test)]
mod tests {

    use std::fs;
    use std::path::Path;

    use uuid::Uuid;

    use super::SqPool;
    use crate::get_artist_pairs;
    use crate::get_lastfm_similar_artists;
    use crate::init_db;

    pub struct TestPool {
        pool: SqPool,
        path: String,
    }

    /// custom `Drop` avoids clogging up your whatever dir when running lots of
    /// tests
    impl Drop for TestPool {
        fn drop(&mut self) { fs::remove_file(&self.path).unwrap(); }
    }

    async fn init_test_db() -> TestPool {
        let id = Uuid::new_v4();
        // let path = format!("/tmp/test-{id}.db");
        let path = format!("test-{id}.db");
        if Path::new(&path).exists() {
            fs::remove_file(&path).unwrap();
        }
        let pool = init_db(&format!("sqlite://{path}")).unwrap();
        sqlx::migrate!().run(&pool).await.unwrap();
        TestPool { pool, path }
    }

    #[tokio::test]
    async fn standard() {
        let t = init_test_db().await;
        assert_eq!(
            get_lastfm_similar_artists("loona", &t.pool)
                .await
                .unwrap()
                .len(),
            100
        );

        let stored = get_artist_pairs("loona", &t.pool).await.unwrap();
        assert_eq!(stored.len(), 100);
        assert_eq!(stored.iter().filter(|e| e.similarity >= 70).count(), 3);
    }

    #[tokio::test]
    async fn special_chars() {
        let t = init_test_db().await;
        assert_eq!(
            get_lastfm_similar_artists("loona 1/3", &t.pool)
                .await
                .unwrap()
                .len(),
            100
        );

        let stored = get_artist_pairs("loona 1/3", &t.pool).await.unwrap();
        assert_eq!(stored.len(), 100);
        assert_eq!(stored.iter().filter(|e| e.similarity >= 70).count(), 3);
    }

    #[tokio::test]
    async fn cached_result() {
        let t = init_test_db().await;
        // TODO: test http requests -- Mock?
        get_lastfm_similar_artists("loona", &t.pool).await.unwrap();
        get_lastfm_similar_artists("loona", &t.pool).await.unwrap();
    }
}
