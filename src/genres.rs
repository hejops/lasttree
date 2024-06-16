use std::fmt::Display;

use anyhow::Context;
use serde::Deserialize;
use serde_json::Value;

use crate::LASTFM_KEY;

/// Wrapper for `Vec<Genre>`, solely for better error-handling
pub struct Genres(pub Vec<Genre>);

// required for error_500
impl Display for Genres {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        for g in self.0.iter() {
            writeln!(f, "{:?}", g)?;
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct Genre {
    pub name: String,
    pub taggings: String,
    pub url: String,
    // reach: String,
    // streamable: String,
}

pub async fn get_json(url: &str) -> anyhow::Result<Value> {
    let resp = reqwest::get(url).await?.text().await?;
    Ok(serde_json::from_str::<Value>(&resp)?)
}

pub async fn get_top_genres() -> anyhow::Result<Genres> {
    // https://www.last.fm/api/show/chart.getTopTags
    let url = format!(
        "https://ws.audioscrobbler.com/2.0/?method=chart.gettoptags&api_key={}&format=json",//&limit=3",
        *LASTFM_KEY
    );
    let json = get_json(&url).await.unwrap();

    let genres: &Vec<Genre> = &json["tags"]["tag"]
        .as_array()
        .context("could not deserialize `tag` array")?
        .iter()
        .map(|g| serde_json::from_value::<Genre>(g.clone()).unwrap())
        .collect();

    // println!("{:#?}", genres);

    // Ok(genres.to_vec())
    Ok(Genres(genres.to_vec()))
}

#[cfg(test)]
mod tests {
    use crate::get_top_genres;

    #[tokio::test]
    async fn test_get_top_genres() {
        let g = get_top_genres().await.unwrap();
        assert_eq!(g.0.len(), 50);
    }
}
