use std::fmt::Display;

use maud::html;
use maud::Markup;
use serde::de;
use serde::Deserialize;
use serde::Deserializer;
use serde_json::Value;
use strum::IntoEnumIterator;

use crate::artists::Artist;
use crate::html;
use crate::utils::build_lastfm_url;
use crate::utils::human_number;
use crate::SqPool;
use crate::LASTFM_KEY;

// TODO: unify User and Chart structs?

pub struct User {
    username: String,
}

#[derive(Deserialize, Debug)]
pub struct Chart {
    #[serde(rename = "artist")]
    artists: Vec<ChartArtist>,

    #[serde(skip)]
    period: Period,
}

#[derive(Deserialize, Debug)]
struct ChartArtist {
    pub name: String,

    #[serde(deserialize_with = "str_to_u64")]
    pub playcount: u64,

    #[serde(rename = "@attr", deserialize_with = "extract_inner")]
    pub rank: u64,
    // last.fm/music/x link, not terribly useful
    // url: String,
}

impl ChartArtist {
    // TODO: this is a poor hack to "inherit" a method from another struct. the
    // proper way to share methods is to use a trait
    async fn get_listeners(
        &self,
        pool: &SqPool,
    ) -> anyhow::Result<u32> {
        Artist::new(&self.name).get_listeners(pool).await
    }
}

impl Chart {
    pub async fn as_html(
        &self,
        user: &str,
        pool: &SqPool,
    ) -> actix_web::Result<Markup> {
        // let library_link = |user: &str, artist: &str| {
        //     format!("https://www.last.fm/user/{user}/library/music/{artist}?date_preset=ALL")
        // };

        // // dropdown redirect requires js, and i don't like the UX anyway
        // // https://stackoverflow.com/questions/7231157
        // let dropdown = html! {
        //     div {
        //         label { "Period " }
        //         select
        //             name=""
        //             hx-get=""
        //             {
        //                 @for p in Period::iter() {
        //                     option value=(p) { (p) }
        //                 }
        //         }
        //     }
        // };

        let periods = html! {
            @for p in Period::iter() {
                @if p == self.period {
                    b { (p) }
                } @else {
                    (html::link(&format!("/charts/{user}/{p}"), &p.to_string()))
                }
                " "
            }
        };

        let html = html! {
            (html::header(&format!("Top artists for {user}")))
            // TODO: total scrobbles

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

            // TODO: htmx tabs?
            // (dropdown)
            (periods)

            table {

                th {"#"}
                th {"Artist"}
                th {"Plays"}
                th {"Listeners"}
                @for artist in &self.artists {
                    @let name = &artist.name;
                    // @let link = library_link(user, name.clone());
                    @let link = format!("/artists/{name}");
                    @let cols = vec![
                        artist.rank.to_string(),
                        (html::link(&link, name).into()),
                        artist.playcount.to_string(),
                        // TODO: plays as % of total plays in the current period
                        human_number(artist.get_listeners(pool).await.unwrap_or(0))
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

// deserializers {{{
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

#[derive(Debug, PartialEq, strum_macros::EnumIter)]
pub enum Period {
    //{{{
    Week,
    Month,
    Quarter,
    Half,
    Year,
    Overall,
}

// impl AsRef<str> for Period {
//     fn as_ref(&self) -> &str { &self.to_string() }
// }

impl Default for Period {
    fn default() -> Self { Self::Overall }
}

impl TryFrom<&str> for Period {
    type Error = String;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let p = match value {
            "7day" => Self::Week,
            "1month" => Self::Month,
            "3month" => Self::Quarter,
            "6month" => Self::Half,
            "12month" => Self::Year,
            "overall" => Self::Overall,
            _ => return Err(format!("Invalid: {value}")),
        };
        Ok(p)
    }
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
//}}}

impl User {
    pub fn new(username: &str) -> anyhow::Result<Self> {
        // TODO: check whether user exists
        let user = User {
            username: username.to_string(),
        };
        Ok(user)
    }

    /// Get an artist chart constrained to one of six fixed time periods, as
    /// defined by last.fm (see `Period` for more details).
    ///
    /// https://www.last.fm/api/show/user.getTopArtists
    pub async fn get_chart_period(
        &self,
        period: Period,
        // limit: u16,
    ) -> anyhow::Result<Chart> {
        let limit = 10;

        let url = build_lastfm_url(
            "user.gettopartists",
            &LASTFM_KEY,
            &[
                ("limit", &limit.to_string()),
                ("period", &period.to_string()),
                ("user", &self.username),
            ],
        )?;

        let json = reqwest::get(url).await?.text().await?;
        let json: Value = serde_json::from_str(&json)?;
        // println!("{:#?}", json);
        let mut chart: Chart = serde_json::from_value(json["topartists"].clone())?;

        // chart.set_period(period);
        chart.period = period;

        Ok(chart)
    }

    /// Unlike `get_chart_period`, this allows a custom window, as specified by
    /// 2 timestamps. If a window is not specified, the last week (7 days) is
    /// used.
    ///
    /// https://www.last.fm/api/show/user.getWeeklyArtistChart
    pub async fn get_chart_window(&self) {
        let _url = build_lastfm_url(
            "user.getweeklyartistchart",
            &LASTFM_KEY,
            &[("user", &self.username)],
        );
    }
}

#[cfg(test)]
mod tests {
    use crate::charts::User;
    use crate::LASTFM_USER;

    #[tokio::test]
    async fn test_week() {
        let ch = User::new(&LASTFM_USER)
            .unwrap()
            .get_chart_period(crate::charts::Period::Week)
            .await
            .unwrap();
        assert_eq!(ch.artists.first().unwrap().rank, 1);
        assert_eq!(ch.artists.last().unwrap().rank, 10);
    }
}
