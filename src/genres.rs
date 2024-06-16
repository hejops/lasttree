use anyhow::Context;
use serde::Deserialize;
use serde_json::Value;

use crate::LASTFM_KEY;

#[derive(Clone, Debug, Deserialize)]
pub struct Genre {
    name: String,
    // reach: String,
    // streamable: String,
    taggings: String,
    url: String,
}

pub async fn get_json(url: &str) -> anyhow::Result<Value> {
    let resp = reqwest::get(url).await?.text().await?;
    Ok(serde_json::from_str::<Value>(&resp)?)
}

pub async fn get_top_genres() -> anyhow::Result<Vec<Genre>> {
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

    Ok(genres.to_vec())
}

#[cfg(test)]
mod tests {
    use crate::get_top_genres;

    #[tokio::test]
    async fn test_get_top_genres() {
        let g = get_top_genres().await.unwrap();
        assert_eq!(g.len(), 50);
    }
}
