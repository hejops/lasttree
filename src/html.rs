use maud::html;
use maud::Markup;
use maud::PreEscaped;
use urlencoding::encode;

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
        // row order must be independent of graph node order
        // TODO: sort table (frontend)
        let mut artists: Vec<&String> = self.nodes.keys().filter(|n| **n != self.root).collect();
        if true {
            artists.sort_by_key(|a| -self.get_child_similarity(a));
        } else {
            artists.sort()
        };

        // Note that the normal pattern of POST/redirect/GET, which is needed to avoid
        // problems with page refresh and form re-submission, is not needed in
        // this case as the POST request doesn’t return a full page.
        //
        // https://github.com/spookylukey/django-htmx-patterns/blob/7aeb17e5ccf3bd4425811fc22b4a26e5d2b23ca2/posts.rst#post-requests
        //
        // importantly, this means that we don't have to create a "spare" GET endpoint,
        // and users are never exposed to it
        let yt_button = |query| {
            html! {
                button
                    hx-post={"/youtube/"(encode(query))}
                    hx-swap="outerHTML"
                    // hx-swap="beforeend"
                    // hx-target={"#"(foo)}
                    // hx-target="#foo"
                    { "YouTube" }
                // p id="foo" {}
            }
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
                            " "
                            (yt_button(artist))
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
                body {
                    h1 { (get_lastfm_url(&self.root)) }
                    (yt_button(&self.root))
                    // https://developer.mozilla.org/en-US/docs/Web/HTML/Element/details
                    // this could be toggled with htmx, but pure html is more elegant
                    details open {
                        summary { "Tree" }
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
