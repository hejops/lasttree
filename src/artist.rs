use std::collections::HashMap;
use std::env;
use std::f64;
use std::str::FromStr;

use anyhow::Context;
use serde::de;
use serde::Deserialize;
use serde::Deserializer;
use serde_json::Value;
use sqlx::sqlite::SqliteConnectOptions;
use sqlx::sqlite::SqlitePoolOptions;
use sqlx::Pool;
use sqlx::Sqlite;

use super::LASTFM_KEY;
use crate::get_artist_from_db;
use crate::store_artist_pair;

/// This struct is only for convenience when we iterate over the json array
#[derive(Deserialize, Debug, Clone)]
pub struct Artist {
    pub name: String,

    /// Preserved as `String`, in order to be able to implement `Eq`
    #[serde(rename = "match", deserialize_with = "str_to_f64")]
    pub similarity: f64,

    pub url: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct LastfmArtist {
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

pub fn init_db(db_url: &str) -> anyhow::Result<sqlx::Pool<Sqlite>> {
    // to enable `sqlx migrate run`, ensure sqlx-cli is installed with the
    // appropriate feature: cargo install sqlx-cli -F rustls,postgres,sqlite[,...]

    // https://github.com/danbruder/twhn_api/blob/689135bf74b007ea88d6ee7e186544e4398619bb/src/main.rs#L29
    let conn = SqliteConnectOptions::from_str(db_url)?
        .create_if_missing(true)
        .optimize_on_close(true, None);
    let pool = SqlitePoolOptions::new().connect_lazy_with(conn);
    Ok(pool)
}

type SqPool = Pool<Sqlite>;

pub async fn get_similar_artists(
    artist: &str,
    pool: &SqPool,
) -> anyhow::Result<HashMap<String, f64>> {
    // let db_url =
    // env::var("DATABASE_URL").unwrap_or("sqlite://lasttree.db".to_owned());
    // let pool = init_db(&db_url)?;

    // // TODO: first check db; if found, build the hashmap without fetching
    // let found = get_artist_from_db(artist, pool).await?;
    // if found.is_some() {
    //     panic!()
    // }

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
    // store_artist(&canon_name, &pool).await?;

    let mut map = HashMap::new();

    for sim in artist.similar_artists {
        // store_artist(&sim.name, &pool).await?;
        store_artist_pair(&canon_name, &sim.name, sim.similarity, pool).await?;
        map.insert(sim.name, sim.similarity);
    }
    // panic!();

    Ok(map)
}

#[cfg(test)]
mod tests {

    use std::fs;

    use super::SqPool;
    use crate::get_artist_pair;
    use crate::get_similar_artists;
    use crate::init_db;

    async fn init_test_db() -> SqPool {
        fs::remove_file("test.db").unwrap();
        let pool = init_db("sqlite://test.db").unwrap();
        sqlx::migrate!().run(&pool).await.unwrap();
        pool
    }

    #[tokio::test]
    async fn lastfm_similar() {
        let pool = init_test_db().await;
        assert_eq!(
            get_similar_artists("loona", &pool).await.unwrap().len(),
            100
        );

        let stored = get_artist_pair("loona", &pool).await.unwrap();
        assert_eq!(stored.len(), 100);
        assert_eq!(stored.iter().filter(|e| e.sim >= 70).count(), 3);
    }
}
