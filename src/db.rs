use std::str::FromStr;

use indexmap::IndexMap;
use sqlx::sqlite::SqliteConnectOptions;
use sqlx::sqlite::SqlitePoolOptions;
use sqlx::Pool;
use sqlx::Sqlite;

use crate::artists::Artist;
use crate::LASTFM_URL;
// use crate::ArtistTree;

pub type SqPool = Pool<Sqlite>;

// TODO: seed db with the 25 most popular artists of a given genre

pub fn init_db(db_url: &str) -> sqlx::Result<SqPool> {
    // to enable `sqlx migrate run`, ensure sqlx-cli is installed with the
    // appropriate feature: cargo install sqlx-cli -F rustls,postgres,sqlite[,...]

    // https://github.com/danbruder/twhn_api/blob/689135bf74b007ea88d6ee7e186544e4398619bb/src/main.rs#L29
    let conn = SqliteConnectOptions::from_str(db_url)?
        .create_if_missing(true)
        .optimize_on_close(true, None);
    let pool = SqlitePoolOptions::new().connect_lazy_with(conn);
    Ok(pool)
}

#[derive(Debug)]
pub struct ArtistPair {
    pub parent: String,
    pub child: String,
    pub similarity: i64,
}

impl Artist {
    //{{{
    /// Query `artists` table (which is much faster than `artist_pairs`)
    pub async fn canonical_name(
        &self,
        pool: &Pool<Sqlite>,
    ) -> sqlx::Result<Option<String>> {
        let lower = self.name.to_lowercase();
        let row = sqlx::query!(
            r#"
            SELECT name FROM artists
            WHERE name_lower = $1
        "#,
            lower,
        )
        .fetch_optional(pool)
        .await?;

        Ok(row.map(|r| r.name))
    }

    pub async fn store(
        &self,
        pool: &Pool<Sqlite>,
    ) -> sqlx::Result<()> {
        let lower = self.name.to_lowercase();
        sqlx::query!(
            r#"
            INSERT OR IGNORE INTO artists (name, name_lower)
            VALUES ($1, $2)
        "#,
            self.name,
            lower,
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn get_artist_pairs(
        &self,
        pool: &Pool<Sqlite>,
    ) -> sqlx::Result<Vec<ArtistPair>> {
        let name = self.canonical_name(pool).await?;

        let mut pairs: Vec<ArtistPair> = sqlx::query!(
            r#"
            SELECT
                parent,
                child,
                similarity
            FROM artist_pairs
            WHERE $1
            -- https://stackoverflow.com/a/13916417
            -- IN (parent_lower, child_lower);
            -- = parent_lower;
            = parent;
        "#,
            name,
        )
        .fetch_all(pool)
        .await?
        .iter()
        .map(|r| ArtistPair {
            parent: r.parent.clone(),
            child: r.child.clone(),
            similarity: r.similarity,
        })
        .collect();

        pairs.sort_by_key(|x| -x.similarity);

        Ok(pairs)
    }

    /// `canon` must be found in `artists` table. This allows the hashmap to be
    /// built without making any network requests.
    pub async fn get_cached_similar_artists(
        &self,
        pool: &SqPool,
    ) -> sqlx::Result<IndexMap<String, i64>> {
        let mut map = IndexMap::new();
        for pair in self.get_artist_pairs(pool).await? {
            map.insert(pair.child, pair.similarity);
        }
        // println!("using cached result");
        Ok(map)
    }

    /// Because sqlite does not support the `NUMERIC` type, `similarity` is cast
    /// to integer before insertion into db.
    pub async fn store_pair(
        &self,
        child: &str,
        similarity: f64,
        pool: &Pool<Sqlite>,
    ) -> sqlx::Result<()> {
        let sim_int = (similarity * 100.0) as u32;
        sqlx::query!(
            r#"
            INSERT OR IGNORE INTO artist_pairs
            (
                parent, -- parent_lower,
                child, -- child_lower,
                similarity,
                date_added
            )
            VALUES ($1, $2, $3, date())
        "#,
            self.name,
            child,
            sim_int
        )
        .execute(pool)
        .await
        .unwrap();
        Ok(())
    }
} //}}}

// {{{
// if self-hosting, a single api key is enough, and we don't need a proper
// login/authentication procedure
pub async fn get_api_key(pool: &Pool<Sqlite>) -> sqlx::Result<Option<String>> {
    let row = sqlx::query!(
        r#"
        SELECT key FROM api_key
        LIMIT 1
        "#,
    )
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|r| r.key))
}

pub async fn store_api_key(
    key: &str,
    pool: &Pool<Sqlite>,
) -> anyhow::Result<()> {
    let key = key.trim();

    // TODO: is there a better dummy request?
    let url = format!(
        "{}&method=chart.gettoptags&api_key={}&limit=1",
        *LASTFM_URL, key
    );
    (reqwest::get(url).await?.status().as_u16() == 200)
        .then_some(())
        .ok_or(anyhow::anyhow!("Invalid API key"))?;

    sqlx::query!(
        r#"
        INSERT INTO api_key
        VALUES ($1)
        "#,
        key
    )
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn delete_api_key(pool: &Pool<Sqlite>) -> sqlx::Result<()> {
    sqlx::query!("DELETE FROM api_key",).execute(pool).await?;
    Ok(())
}
//}}}
