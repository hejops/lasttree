use maud::html;
use maud::Markup;
use maud::PreEscaped;
use urlencoding::encode;

use crate::ArtistTree;
use crate::APP_NAME;

// pub fn svg(
//     artist: &str,
//     raw_svg: &str,
// ) -> Markup {
//     // https://github.com/bigskysoftware/htmx/blob/master/README.md#quick-start
//     // https://stackoverflow.com/a/77994867
//
//     // interestingly, maud will render htmx in a box, with monospace font.
//     // potentially, this may be circumvented if css is applied?
//     html! {
//         script src="https://unpkg.com/htmx.org/dist/htmx.min.js" {}
//
//         button
//             hx-post={"/artists/"(artist)"/svg"}
//             name="svg"
//             value=(raw_svg)
//             hx-swap="/outerHTML"
//             { "Show graph (SVG)" }
//     }
// }

// this could be an ArtistTree method, but only if this gets used a lot
pub fn get_lastfm_url(name: &str) -> Markup {
    // https://github.com/isgasho/lettre/blob/a0980d017b1257018446228162a8d17bff17798f/examples/maud_html.rs#L24
    html! {
        a href=(format!("https://last.fm/music/{name}")) { (name) }
    }
}

pub fn link(
    path: &str,
    label: &str,
) -> Markup {
    html! { a href=(path) { (label) } }
}

pub fn table_row(s: &str) -> Markup {
    html! {
        tr {
            td { (PreEscaped(s)) }
        }
    }
}

pub fn list_item(s: &str) -> Markup {
    html! {
        li { (PreEscaped(s)) }
    }
}

impl ArtistTree {
    pub async fn as_html(&self) -> anyhow::Result<Markup> {
        // TODO: extract youtube audio link (yt-dlp), then embed
        // but this is useless unless i can customise it to show only the
        // button/progress
        // https://developer.mozilla.org/en-US/docs/Web/HTML/Element/audio

        //       // https://developers.google.com/youtube/player_parameters
        //       let player = r#"<iframe id="ytplayer" type="text/html" width="50"
        // height="50" src="https://www.youtube.com/embed/M7lc1UVf-VE?autoplay=0&origin=http://example.com"
        // frameborder="0"></iframe>"#;

        let player = "";

        // row order must be independent of graph node order
        // TODO: sort table (frontend)
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
                title { (APP_NAME.to_string())": "(self.root) }
                // a href=("/") { "Home" }
                (link("/", "Home"))
                (PreEscaped(player))
                body {
                    // h1 { (self.root) }
                    h1 { (get_lastfm_url(&self.root)) }
                    // https://developer.mozilla.org/en-US/docs/Web/HTML/Element/details
                    details open {
                        summary { "Graph" }
                        // (PreEscaped(raw_svg))
                        (PreEscaped(&self.as_svg()))
                    }
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
    use crate::html::get_lastfm_url;
    use crate::html::table_row;

    #[test]
    fn table() {
        let x = "loona";
        let link = get_lastfm_url(x);
        assert_eq!(
            &link.clone().into_string(),
            r#"<a href="https://last.fm/music/loona">loona</a>"#
        );
        let row = table_row(&link.into_string());
        assert_eq!(
            row.into_string(),
            r#"<tr><td><a href="https://last.fm/music/loona">loona</a></td></tr>"#
        );
    }
}
