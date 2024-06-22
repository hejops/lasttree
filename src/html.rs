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
                        required
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

pub fn header(page_title: &str) -> Markup {
    html! {
        title { (APP_NAME.as_str())" - "(page_title) }
        (link("/", "Home"))
        h2 { (page_title) }
    }
}

/// <tr><td>
// pub fn table_row(cols: Vec<Markup>) -> Markup {
// arg should be Vec<String>, not Vec<Markup>; Markup -> String is easy, while
// String -> Markup is more tedious
pub fn table_row(cols: Vec<String>) -> Markup {
    html! {
        tr {
            @for s in cols {
                td { (PreEscaped(s)) }
                // td { (s) }
            }
        }
    }
}

pub fn list_item(s: &str) -> Markup {
    html! {
        li { (PreEscaped(s)) }
    }
}

pub fn spinner() -> Markup {
    use std::fs;

    use base64::engine::general_purpose;
    use base64::Engine;

    // https://github.com/Techcable/smstools/blob/a49d5c507333359e93e8a2e2bed63666e8dea145/src/html.rs#L88
    let spinner = fs::read("./img/spinner.svg").unwrap();
    let spinner = format!(
        "data:image/svg+xml;base64,{}",
        general_purpose::STANDARD.encode(spinner)
    );
    html! { (spinner) }
}

impl ArtistTree {
    pub async fn as_html(&self) -> anyhow::Result<Markup> {
        // row order must be independent of graph node order
        let mut artists: Vec<&String> = self.nodes.keys().filter(|n| **n != self.root).collect();
        artists.sort_by_key(|a| -self.get_child_similarity(a));

        // if true {
        //     artists.sort_by_key(|a| -self.get_child_similarity(a));
        // } else {
        //     artists.sort()
        // };

        // TODO: /artists/X/json endpoint to re-export youtube results (for piping into
        // mpv)
        // TODO: new column + button to "ignore" artists (new table)

        // Note that the normal pattern of POST/redirect/GET, which is needed to avoid
        // problems with page refresh and form re-submission, is not needed in
        // this case as the POST request doesn’t return a full page.
        //
        // https://github.com/spookylukey/django-htmx-patterns/blob/7aeb17e5ccf3bd4425811fc22b4a26e5d2b23ca2/posts.rst#post-requests
        //
        // importantly, this means that we don't have to create a "spare" GET endpoint,
        // and users are never exposed to it
        // https://htmx.org/examples/active-search/
        // https://htmx.org/attributes/hx-swap/
        // let yt_button = |query| {
        fn yt_button(query: &str) -> Markup {
            html! {
                button
                    hx-post={"/youtube/"(encode(query))}
                    hx-trigger="click" // send post request only on click
                    hx-swap="innerHTML" // if outerHTML, once #player is replaced, it cannot be
                                        // replaced again!
                    hx-target="#player" // must be #, not .
                    hx-indicator=".htmx-indicator"
                { "YouTube" }
            }
        }

        // TODO: right align Similarity values (but not header)
        // https://stackoverflow.com/a/1332648

        // https://stackoverflow.com/a/49012896
        // https://users.rust-lang.org/t/closure-lifetime-issue-cast-requires-1-must-outlive-static/84247/2
        // use a [(&str, fn)] to generate table, though this requires a bit of lifetime
        // gymnastics
        type ColumnGenerator<'a> = (&'a str, Box<dyn Fn(&str) -> String + 'a>);
        let cols: Vec<ColumnGenerator> = vec![
            (
                "Similarity",
                Box::new(|artist| self.get_child_similarity(artist).to_string()),
            ),
            (
                "Artist",
                Box::new(|artist| {
                    link(&format!("/artists/{}", encode(artist)), artist).into_string()
                }),
            ),
            (
                "Links",
                Box::new(|artist| {
                    format!(
                        "{} {}",
                        link(&format!("https://last.fm/music/{artist}"), "Last.fm").into_string(),
                        yt_button(artist).into_string(),
                    )
                }),
            ),
        ];

        let table = html! {
            table {

                // th { "Similarity" }
                // th { "Artist" }
                // th { "Links" }
                // @for artist in artists {
                //     @let cols = vec![
                //         self.get_child_similarity(artist).to_string(),
                //         link(&format!("/artists/{}", encode(artist)), artist).into(),
                //         (format!("{} {}",
                //             link(&format!("https://last.fm/music/{artist}"), "Last.fm").into_string(),
                //             yt_button(artist).into_string(),
                //         ))
                //     ];
                // }

                @for (c, _) in cols.iter() { th { (c) } }
                @for artist in artists {
                    // @let cols: Vec<Markup> = cols.iter().map(|x| &x.1).map(|f| f(artist)).collect();
                    // @let cols = cols.iter().map(|x| (x.1)(artist));
                    (table_row(cols.iter().map(|x| (x.1)(artist)).collect()))
                }
            }
        };

        let html = html! {
            html {
                script src="https://unpkg.com/htmx.org@1.9.12" {}
                style {
                    "table, th, td { border: 1px solid grey; }"
                }
                (header(&format!("Artist: {}", self.root)))
                // TODO: show artist's tags (with links?)
                // h1 { (get_lastfm_url(&self.root)) }
                body {
                    (yt_button(&self.root))
                    // https://developer.mozilla.org/en-US/docs/Web/HTML/Element/details
                    // this could be toggled with htmx, but pure html is more elegant
                    details open {
                        summary { "Tree" }
                        (PreEscaped(&self.as_svg()))
                    }
                    span class="htmx-indicator" {
                        img width="20" src=(spinner()) {}
                        // TODO: inject value from yt_button into this string?
                        "Searching..."
                    }
                    span id="player" { }
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
    fn test_link() {
        let x = "loona";
        let link = get_lastfm_url(x);

        assert_eq!(
            &link.clone().into_string(),
            r#"<a href="https://last.fm/music/loona">loona</a>"#
        );

        let cols = ["1".to_string(), link.into_string()];
        let row = table_row(cols.to_vec());
        assert_eq!(
            row.into_string(),
            r#"<tr><td>1</td><td><a href="https://last.fm/music/loona">loona</a></td></tr>"#
        );
    }
}
