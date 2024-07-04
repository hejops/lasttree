// will eventually be deprecated

use std::fmt::Display;

use serde::Deserialize;
use serde_json::Value;

use crate::utils::build_lastfm_url;
use crate::LASTFM_KEY;

/// Wrapper for `Vec<Genre>`, solely for better error-handling
#[derive(Deserialize)]
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

#[derive(Debug, Deserialize)]
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

/// https://www.last.fm/api/show/chart.getTopTags
// nearly identical to tag.getTopTags
pub async fn get_top_genres() -> anyhow::Result<Genres> {
    let url = build_lastfm_url("chart.gettoptags", &LASTFM_KEY, &[])?;
    let json = get_json(url.as_ref()).await.unwrap();

    let genres = serde_json::from_value(json["tags"]["tag"].clone())?;

    Ok(genres)
}

pub async fn get_genre() {
    let url = build_lastfm_url("tag.gettopartists", &LASTFM_KEY, &[]).unwrap();
    let json = get_json(url.as_ref()).await.unwrap();
    println!("{:?}", json);

    // let genres = serde_json::from_value(json["tags"]["tag"].clone())?;
    // Ok(genres)
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
