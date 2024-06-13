use itertools::Itertools;
use maud::html;
use maud::Markup;
use maud::PreEscaped;
use urlencoding::encode;

use crate::ArtistTree;
use crate::DotOutput;

pub fn get_lastfm_url(name: &str) -> Markup {
    // https://github.com/isgasho/lettre/blob/a0980d017b1257018446228162a8d17bff17798f/examples/maud_html.rs#L24
    html! {
        a href=(format!("https://last.fm/music/{name}")) { (name) }
    }
}

pub fn table_row(s: String) -> Markup {
    html! {
        tr {
            td { (PreEscaped(s)) }
        }
    }
}

pub fn list_item(s: String) -> Markup {
    html! {
        li { (PreEscaped(s)) }
    }
}

impl ArtistTree {
    pub async fn as_html(&self) -> anyhow::Result<Markup> {
        let svg = self
            .as_dot(DotOutput::Svg)
            .await?
            .lines()
            .skip(3)
            .join("\n");

        // order should be independent of graph node order
        // TODO: sort method via url param (htmx idea?)
        let mut artists: Vec<&String> = self.nodes.keys().filter(|n| **n != self.root).collect();
        if true {
            artists.sort_by_key(|a| -self.get_child_similarity(a));
        } else {
            artists.sort()
        };

        let html = html! {
            html {
                style {
                    "table, th, td { border: 1px solid grey; }"
                }
                title { "lasttree: "(self.root) }
                a href=("/") { "Home" }
                body {
                    // h1 { (self.root) }
                    h1 {
                        a href=(format!("https://last.fm/music/{}", self.root))
                        { (self.root) }
                    }
                    (PreEscaped(svg))
                }
                table {
                    th { "Artist" }
                    th { "Similarity" }
                    th { "Last.fm" }
                    @for artist in artists {
                        tr {
                            td { a href=(format!("/artists/{}", encode(artist))) { (artist) } }
                            td { (self.get_child_similarity(artist)) }
                            td { a href=(format!("https://last.fm/music/{artist}")) { "↪" } }
                        }
                    }
                }
            }
        };

        Ok(html)
    }
}

#[cfg(test)]
mod tests {
    use crate::get_lastfm_url;
    use crate::table_row;

    #[test]
    fn table() {
        let x = "loona";
        let link = get_lastfm_url(x);
        assert_eq!(
            link.clone().into_string(),
            r#"<a href="https://last.fm/music/loona">loona</a>"#
        );
        let row = table_row(link.into());
        assert_eq!(
            row.into_string(),
            r#"<tr><td><a href="https://last.fm/music/loona">loona</a></td></tr>"#
        );
    }
}
