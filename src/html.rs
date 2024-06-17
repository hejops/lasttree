use maud::html;
use maud::Markup;
use maud::PreEscaped;
use urlencoding::encode;

use crate::player::get_youtube_audio_link;
use crate::player::search_youtube;
use crate::ArtistTree;
use crate::APP_NAME;

pub fn api_key_form(redirect_to: &str) -> Markup {
    //{{{
    html! {
        p {
            "A Last.fm API key is required. "
            "Click " (link("https://www.last.fm/api", "here")) " to get one."
        }
        form
            method="POST"
            action="/login"
            {
                label { "API key: "
                    input
                        type="password"
                        name="key"
                        { }
                    button type="submit" { "Submit" }
                }

                input hidden
                    type="text"
                    name="redirect_to"
                    value=(redirect_to) {}

            }

    }
} //}}}

// this could be an ArtistTree method, but only if this gets used a lot
pub fn get_lastfm_url(name: &str) -> Markup {
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

pub fn header() -> Markup {
    html! {
        (link("/", "Home"))
    }
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
        // yt embed would be the simplest option, but it is not very useful, unless i
        // can customise it to show only the button/progress (which was possible
        // way back in like 2009)
        // https://developers.google.com/youtube/player_parameters

        // https://developer.mozilla.org/en-US/docs/Web/HTML/Element/audio

        let player = html! {
            // audio controls {
            //     source src=(search_youtube(&self.root).await.unwrap()) {}
            // }
            button type="submit" { "Search" }
            // htmx replace button
        };

        // row order must be independent of graph node order
        // TODO: sort table (frontend)
        let mut artists: Vec<&String> = self.nodes.keys().filter(|n| **n != self.root).collect();
        if true {
            artists.sort_by_key(|a| -self.get_child_similarity(a));
        } else {
            artists.sort()
        };

        // TODO: right align Similarity values (but not header)
        // https://stackoverflow.com/a/1332648

        let table = html! {
            table {
                th { "Similarity" }
                th { "Artist" }
                th { "Links" }
                // th { "YouTube" }
                @for artist in artists {
                    tr {
                        td { (self.get_child_similarity(artist)) }
                        td { (link(&format!("/artists/{}", encode(artist)), artist)) }
                        td {
                            (link(&format!("https://last.fm/music/{artist}"), "Last.fm" ))
                            // button hx-get="/" hx-swap="outerHTML" { "Last.fm" }
                            " "
                            button hx-get="/" hx-swap="outerHTML" { "YouTube" }
                        }
                        // td { (player) }
                    }
                }
            }
        };

        let html = html! {
            html {
                script src="https://unpkg.com/htmx.org@1.9.12" {}
                style {
                    "table, th, td { border: 1px solid grey; }"
                }
                title { (APP_NAME.to_string())": "(self.root) }
                // a href=("/") { "Home" }
                (link("/", "Home"))
                // (player)
                body {
                    // h1 { (self.root) }
                    h1 { (get_lastfm_url(&self.root)) }
                    // https://developer.mozilla.org/en-US/docs/Web/HTML/Element/details
                    details open {
                        summary { "Tree" }
                        // (PreEscaped(raw_svg))
                        (PreEscaped(&self.as_svg()))
                    }
                    (table)
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
