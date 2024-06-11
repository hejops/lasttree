// TODO: represent in sqlite --
// 1 table for artist/id,
// 1 table for (id,id)/sim

use sqlx::Pool;
use sqlx::Sqlite;

use crate::Edge;

pub async fn get_artist_from_db(
    name: &str,
    pool: &Pool<Sqlite>,
) -> anyhow::Result<Option<String>> {
    let lower = name.to_lowercase();
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

pub async fn store_artist(
    name: &str,
    pool: &Pool<Sqlite>,
) -> anyhow::Result<()> {
    let lower = name.to_lowercase();
    sqlx::query!(
        r#"
            INSERT OR IGNORE INTO artists (name, name_lower)
            VALUES ($1, $2)
        "#,
        name,
        lower,
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn get_artist_pair(
    name: &str,
    pool: &Pool<Sqlite>,
) -> anyhow::Result<Vec<Edge>> {
    let lower = name.to_lowercase();

    let pairs = sqlx::query!(
        r#"
            SELECT
                parent,
                child,
                similarity
            FROM artist_pairs
            WHERE $1 IN (parent_lower, child_lower); -- https://stackoverflow.com/a/13916417
        "#,
        lower,
    )
    .fetch_all(pool)
    .await?
    .iter()
    .map(|r| Edge {
        parent: r.parent.clone(),
        child: r.child.clone(),
        sim: r.similarity,
    })
    .collect();

    Ok(pairs)
}

/// Because sqlite does not support the `NUMERIC` type, `similarity` is cast to
/// integer before insertion into db.
pub async fn store_artist_pair(
    name1: &str,
    name2: &str,
    similarity: f64,
    pool: &Pool<Sqlite>,
) -> anyhow::Result<()> {
    let lower1 = name1.to_lowercase();
    let lower2 = name2.to_lowercase();
    let sim_int = (similarity * 100.0) as u32;
    sqlx::query!(
        r#"
            INSERT OR IGNORE INTO artist_pairs 
            (
                parent, parent_lower,
                child, child_lower,
                similarity
            )
            VALUES ($1, $2, $3, $4, $5)
        "#,
        name1,
        lower1,
        name2,
        lower2,
        sim_int
    )
    .execute(pool)
    .await
    .unwrap();
    Ok(())
}
