use serde::de;
use serde::Deserialize;
use serde::Deserializer;
use serde_json::Value;

use crate::LASTFM_KEY;
use crate::LASTFM_USER;

#[derive(Deserialize, Debug)]
pub struct Chart {
    #[serde(rename = "artist")]
    pub artists: Vec<ChartArtist>,
}

#[derive(Deserialize, Debug)]
pub struct ChartArtist {
    pub name: String,

    #[serde(deserialize_with = "str_to_u64")]
    pub playcount: u64,

    #[serde(rename = "@attr", deserialize_with = "extract_inner")]
    pub rank: u64,
    // last.fm/music/x link, not terribly useful
    // url: String,
}

fn str_to_u64<'de, D>(deserializer: D) -> Result<u64, D::Error>
where
    D: Deserializer<'de>,
{
    let val = match Value::deserialize(deserializer)? {
        Value::String(s) => s.parse().map_err(de::Error::custom)?,
        Value::Number(num) => num
            .as_u64()
            .ok_or_else(|| de::Error::custom("Invalid number"))?,
        _ => return Err(de::Error::custom("wrong type")),
    };
    Ok(val)
}

fn extract_inner<'de, D>(deserializer: D) -> Result<u64, D::Error>
where
    D: Deserializer<'de>,
{
    match Value::deserialize(deserializer)? {
        Value::Object(obj) => {
            // let x = &obj["rank"];
            // println!("{:?}", x); // String("...")
            // println!("{:?}", x.as_u64()); // does not parse into u64!

            let val: u64 = obj["rank"]
                .as_str()
                .unwrap()
                .parse()
                .map_err(de::Error::custom)?;
            Ok(val)

            // match &obj["rank"] {
            //     Value::String(s) =>
            // Ok(s.parse().map_err(de::Error::custom)?),
            //     _ => Err(de::Error::custom("wrong type")),
            // }
        }
        _ => Err(de::Error::custom("wrong type")),
    }
}

// TODO: charts should be displayed as tabs (1 for each timeframe)

pub async fn week() -> anyhow::Result<Chart> {
    let url = format!(
        "http://ws.audioscrobbler.com/2.0/?method=user.gettopartists&user={}&api_key={}&format=json",//&limit=3",
        *LASTFM_USER,
        *LASTFM_KEY
    );
    let json = reqwest::get(url).await?.text().await?;
    let json: Value = serde_json::from_str(&json)?;
    let chart = serde_json::from_value(json["topartists"].clone())?;
    // println!("{:#?}", json);
    Ok(chart)
}

pub async fn overall() {
    let _url = format!("http://ws.audioscrobbler.com/2.0/?method=user.getweeklyartistchart&user={}&api_key={}&format=json",
        *LASTFM_USER,
        *LASTFM_KEY
    );
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn test_week() {
        let ch = crate::charts::week().await.unwrap();
        assert_eq!(ch.artists.first().unwrap().rank, 1);
    }
}
