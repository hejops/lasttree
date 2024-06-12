use itertools::Itertools;
use maud::html;
use maud::Markup;
use maud::PreEscaped;

use crate::ArtistTree;
use crate::DotOutput;

// TODO: return Markup
pub fn get_lastfm_url(name: &str) -> String {
    // https://github.com/isgasho/lettre/blob/a0980d017b1257018446228162a8d17bff17798f/examples/maud_html.rs#L24
    html! {
        a href=(format!("https://last.fm/music/{name}")) { (name) }
    }
    .into_string()
}

pub fn table_row(s: String) -> String {
    html! {
        tr { td { (PreEscaped(s)) } }
    }
    .into_string()
}

pub fn list_item(s: String) -> String {
    html! {
        li { (PreEscaped(s)) }
    }
    .into_string()
}

impl ArtistTree {
    pub async fn as_html(&self) -> anyhow::Result<Markup> {
        let svg = self
            .as_dot(DotOutput::Svg)
            .await?
            .lines()
            .skip(3)
            .join("\n");

        // println!("{:#?}", self.graph);

        let links = self
            .nodes
            .keys()
            .filter(|n| **n != self.root)
            .sorted() // does not affect graph
            // .map(self.to_owned)
            .map(|x| get_lastfm_url(x))
            .map(table_row)
            .join("\n");

        let html = html! {
            html {
                body { h1 { (self.root.clone()) } (PreEscaped(svg)) }
                table {
                    th { "foo" }
                    (PreEscaped(links))
                }
            }
        };

        Ok(html)
    }
}
