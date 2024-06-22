use std::fmt::Display;

use maud::html;
use maud::Markup;
use serde::de;
use serde::Deserialize;
use serde::Deserializer;
use serde_json::Value;
use strum::IntoEnumIterator;

use crate::html;
use crate::LASTFM_KEY;

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

impl Chart {
    pub fn as_html(
        &self,
        user: &str,
    ) -> actix_web::Result<Markup> {
        // TODO: charts should be displayed as tabs (1 for each period), or dropdown

        // <div>
        //     <label >Make</label>
        //     <select name="make" hx-get="/models" hx-target="#models"
        // hx-indicator=".htmx-indicator">       <option value="audi">Audi</option>
        //       <option value="toyota">Toyota</option>
        //       <option value="bmw">BMW</option>
        //     </select>
        //   </div>
        let dropdown = html! {
            div {
                label { "Period" }
                select
                    name=""
                    hx-get=""
                    {
                        @for p in Period::iter() {
                            option value=(p) { (p) }
                        }

                }
            }
        };

        let html = html! {
            (html::header(&format!("Top artists for {user}")))

            form //{{{
                method="POST"
                action="/charts" // target
                {
                label { "Search user: " // form_label
                    input
                        required
                        type="text"
                        autofocus="true"
                        name="user" // field
                        { }
                    button type="submit" { "Search" } // button_label
                }
            }//}}}

            (dropdown)

            table {

                th {"#"}
                th {"Artist"}
                th {"Plays"}
                @for artist in &self.artists {
                    @let name = &artist.name;
                    // @let link = library_link(user, name.clone());
                    @let link = format!("/artists/{name}");
                    @let cols = vec![
                        artist.rank.to_string(),
                        (html::link(&link, name).into()),
                        artist.playcount.to_string(),
                    ];
                    (html::table_row(cols))
                }

                // @for (c, _) in cols.iter() { th { (c) } }
                // @for artist in artists {
                //     (table_row(cols.iter().map(|x| (x.1)(artist)).collect()))
                // }

                // TODO: final row to load next page (hx-swap="afterend")
                // https://htmx.org/examples/click-to-load/
            }
        };
        Ok(html)
    }
}

//{{{
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
//}}}

#[derive(strum_macros::EnumIter)]
pub enum Period {
    Overall,
    Week,
    Month,
    Quarter,
    Half,
    Year,
}

impl Display for Period {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Week => "7day",
                Self::Month => "1month",
                Self::Quarter => "3month",
                Self::Half => "6month",
                Self::Year => "12month",
                Self::Overall => "overall",
            }
        )
    }
}

pub struct User {
    username: String,
}

impl User {
    pub fn new(username: &str) -> anyhow::Result<Self> {
        // TODO: check whether user exists
        let user = User {
            username: username.to_string(),
        };
        Ok(user)
    }

    pub async fn overall(
        // username: &str,
        &self,
        period: Option<Period>,
    ) -> anyhow::Result<Chart> {
        let period = period.unwrap_or(Period::Overall);
        // https://www.last.fm/api/show/user.getWeeklyArtistChart -- week, or custom start+end
        // https://www.last.fm/api/show/user.getTopArtists -- allows fixed periods
        let url = format!(
        // "http://ws.audioscrobbler.com/2.0/?method=user.getweeklyartistchart&user={}&api_key={}&format=json&limit=3",
        "http://ws.audioscrobbler.com/2.0/?method=user.gettopartists&user={}&api_key={}&period={period}&format=json",//&limit=3",
        self.username,
        *LASTFM_KEY
    );
        let json = reqwest::get(url).await?.text().await?;
        let json: Value = serde_json::from_str(&json)?;
        println!("{:#?}", json);
        let chart = serde_json::from_value(json["topartists"].clone())?;
        Ok(chart)
    }
}

pub async fn weekly(user: &str) {
    let _url = format!("http://ws.audioscrobbler.com/2.0/?method=user.getweeklyartistchart&user={user}&api_key={}&format=json",
        *LASTFM_KEY
    );
}

#[cfg(test)]
mod tests {
    use crate::charts::User;
    use crate::LASTFM_USER;

    #[tokio::test]
    async fn test_week() {
        // let ch = crate::charts::overall(&LASTFM_USER, None).await.unwrap();
        let ch = User::new(&LASTFM_USER)
            .unwrap()
            .overall(None)
            .await
            .unwrap();
        assert_eq!(ch.artists.first().unwrap().rank, 1);
    }
}
