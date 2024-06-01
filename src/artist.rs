use std::f64;

use anyhow::Context;
use anyhow::Result;
use serde::Deserialize;
use serde_json::Value;

use super::LASTFM_KEY;

/// This struct is quite poorly implemented
#[derive(Deserialize, Debug, Clone)]
pub struct SimilarArtist {
    pub name: String,
    /// Preserved as `String`, in order to be able to implement `Eq`
    #[serde(rename = "match")]
    pub similarity: String,
}

impl SimilarArtist {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string().to_lowercase(),
            similarity: "1.0".to_string(),
        }
    }

    pub fn sim_gt(
        &self,
        x: f64,
    ) -> bool {
        self.similarity.parse::<f64>().unwrap() > x
    }

    pub fn get_similar(&self) -> Result<Vec<SimilarArtist>> {
        let url = format!(
            "http://ws.audioscrobbler.com/2.0/?method=artist.getsimilar&artist={}&api_key={}&format=json", 
            self.name,
            *LASTFM_KEY
        );

        let resp = reqwest::blocking::get(url)?.text()?;
        let raw_json: Value = serde_json::from_str(&resp)?;

        let sim = raw_json
            .get("similarartists")
            .context("no similarartists")?
            .get("artist")
            .unwrap();

        Ok(serde_json::from_value::<Vec<SimilarArtist>>(sim.clone())?)
    }
}
