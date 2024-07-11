use std::str::FromStr;

use serde_json::json;
use sqlx::sqlite::SqliteConnectOptions;
use sqlx::sqlite::SqlitePoolOptions;
use sqlx::Pool;
use sqlx::Sqlite;

use crate::artists::Artist;
use crate::utils::build_lastfm_url;

pub type SqPool = Pool<Sqlite>;

// TODO: on "error: migration ... was previously applied but is missing in the
// resolved migrations" (on deleting an .sql file which was migrated) -- may be
// possible to avoid nuking the db by "delet[ing] the version row from the
// _sqlx_migration table"
//
// https://old.reddit.com/r/rust/comments/12z6n77/resolving_previously_applied_but_missing_error_in/l0qlcz6/

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
    pub async fn canonical_name(
        &self,
        pool: &SqPool,
    ) -> sqlx::Result<Option<String>> {
        let lower = self.name.to_lowercase();
        let row = sqlx::query!(
            r#"
            SELECT name FROM artists
            -- WHERE name = $1 COLLATE NOCASE
            WHERE name_lower = $1
        "#,
            lower,
        )
        .fetch_optional(pool)
        .await?;

        Ok(row.map(|r| r.name))
    }

    /// Add to db, skipping if it already exists. `canon_name` should be derived
    /// by parsing last.fm json.
    pub async fn store(
        &self,
        pool: &SqPool,
        canon_name: &str,
    ) -> sqlx::Result<()> {
        let lower = canon_name.to_lowercase();
        sqlx::query!(
            r#"
            INSERT OR IGNORE INTO artists (name, name_lower)
            VALUES ($1, $2)
            -- INSERT OR IGNORE INTO artists (name)
            -- VALUES ($1)
        "#,
            canon_name,
            lower,
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    // listeners {{{
    pub async fn store_listeners(
        &self,
        pool: &SqPool,
        listeners: u32,
    ) -> sqlx::Result<()> {
        let name = self.name.to_lowercase();
        sqlx::query!(
            r#"
            UPDATE artists
            SET listeners = $1
            -- WHERE name = $2 COLLATE NOCASE
            WHERE name_lower = $2
            "#,
            listeners,
            name
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn get_listeners_db(
        &self,
        pool: &SqPool,
    ) -> sqlx::Result<Option<i64>> {
        let name = self.name.to_lowercase();
        let row = sqlx::query!(
            r#"
            -- ! does away with double Option
            -- https://docs.rs/sqlx/latest/sqlx/macro.query.html#force-not-null
            -- what happens if column -is- null? idk
            SELECT listeners as "listeners!"
            FROM artists
            -- WHERE name = $1 COLLATE NOCASE
            WHERE name_lower = $1
            "#,
            name
        )
        .fetch_optional(pool)
        .await?;
        Ok(row.map(|r| r.listeners))
    }
    //}}}

    // similars {{{
    /// Return pairs in descending similarity
    pub async fn get_artist_pairs(
        &self,
        pool: &SqPool,
    ) -> sqlx::Result<Option<Vec<ArtistPair>>> {
        let name = self.canonical_name(pool).await?;

        let mut pairs: Vec<ArtistPair> = sqlx::query!(
            r#"
            SELECT
                parent,
                child,
                similarity
            FROM artist_pairs
            -- WHERE parent = $1 COLLATE NOCASE
            WHERE parent = $1
        "#,
            name,
        )
        // note: the `Vec` returned may be empty
        .fetch_all(pool)
        .await?
        .iter()
        .map(|r| ArtistPair {
            parent: r.parent.clone(),
            child: r.child.clone(),
            similarity: r.similarity,
        })
        .collect();

        Ok(match pairs.is_empty() {
            true => None,
            false => {
                pairs.sort_by_key(|x| -x.similarity);
                Some(pairs)
            }
        })
    }

    /// `parent` and `child` must both be canonical names.
    ///
    /// Because sqlite does not support the `NUMERIC` type, `similarity` is cast
    /// to integer before insertion into db.
    pub async fn store_pair(
        &self,
        pool: &SqPool,
        parent: &str,
        child: &str,
        similarity: f64,
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
            parent,
            child,
            sim_int
        )
        .execute(pool)
        .await
        .unwrap();
        Ok(())
    } //}}}

    /// Because `serde_json::json!` is used for json serialisation, SQLite's
    /// `json()` is unnecessary
    // https://www.sqlite.org/json1.html#jmini
    pub async fn store_tags(
        &self,
        pool: &SqPool,
        tags: &Vec<String>,
    ) -> sqlx::Result<()> {
        // let canon = self.canonical_name(pool).await?.context("fjdaks").unwrap();
        // self.store(pool, &canon).await?;

        let name = self.name.to_lowercase();
        let tags = json!(tags);

        sqlx::query!(
            r#"
            UPDATE artists
            SET tags = $1
            -- WHERE name = $2 COLLATE NOCASE
            WHERE name_lower = $2
            "#,
            tags,
            name
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn get_tags_db(
        &self,
        pool: &SqPool,
    ) -> sqlx::Result<Option<Vec<String>>> {
        let name = self.name.to_lowercase();

        let row = sqlx::query!(
            r#"
            -- this is how to select sqlite json properly
            -- (otherwise 'unsupported type NULL')
            -- https://docs.rs/sqlx/latest/sqlx/macro.query.html#force-a-differentcustom-type
            -- note: deserialized must still be done separately
            SELECT tags as "tags!: serde_json::Value"
            FROM artists
            -- WHERE name = $1 COLLATE NOCASE
            WHERE name_lower = $1
            "#,
            name
        )
        .fetch_optional(pool)
        .await?;

        Ok(row.map(|row| serde_json::from_value::<Vec<String>>(row.tags).unwrap()))
    }
}

pub async fn get_random_artist(pool: &SqPool) -> sqlx::Result<Option<String>> {
    let row = sqlx::query!(
        r#"
        SELECT name FROM artists
        ORDER BY RANDOM() LIMIT 1
        "#,
    )
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|r| r.name))
}

// api key {{{
// if self-hosting, a single api key is enough, and we don't need a proper
// login/authentication procedure
pub async fn get_api_key(pool: &SqPool) -> sqlx::Result<Option<String>> {
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
    pool: &SqPool,
    key: &str,
) -> anyhow::Result<()> {
    let key = key.trim();

    // TODO: is there a better dummy request?
    let url = build_lastfm_url("chart.gettoptags", key, &[("limit", "1")])?;
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

pub async fn delete_api_key(pool: &SqPool) -> sqlx::Result<()> {
    sqlx::query!("DELETE FROM api_key",).execute(pool).await?;
    Ok(())
}
//}}}

#[cfg(test)]
mod tests {
    use crate::artists::Artist;
    use crate::tests::TestPool;
    use crate::LASTFM_KEY;

    #[tokio::test]
    async fn tags() {
        let pool = &TestPool::new(Some(&LASTFM_KEY)).await.pool;
        let a = Artist::new("foo");

        a.store(pool, "Foo").await.unwrap();

        let tags: Vec<String> = vec!["A", "B", "C"]
            .into_iter()
            .map(|s| s.to_owned())
            .collect();

        a.store_tags(pool, &tags).await.unwrap();
        assert_eq!(tags, a.get_tags_db(pool).await.unwrap().unwrap());
    }
}
