use actix_web::get;
use actix_web::post;
use actix_web::web;
use actix_web::HttpResponse;
use actix_web::Responder;
use maud::html;
use maud::Markup;
use serde::Deserialize;

use crate::charts;
use crate::error_500;
use crate::get_api_key;
use crate::get_top_genres;
use crate::html;
// should not conflict with maud::html
use crate::store_api_key;
use crate::ArtistTree;
use crate::SqPool;
use crate::APP_NAME;

// as far as possible, this file should not contain overly complicated markup;
// simple markup is still ok for locality of behaviour

async fn redirect(path: &str) -> impl Responder {
    HttpResponse::SeeOther()
        .insert_header(("Location", path))
        .finish()
}

pub async fn not_found() -> impl Responder { redirect("/").await }

#[get("/")]
async fn home() -> actix_web::Result<Markup> {
    let html = html! {
        h1 { (APP_NAME.to_string()) }
        h2 { "Home" }
        ul {
            li { (html::link("/artists", "Artists")) }
            // li { (html::link("/genres", "Genres")) }
            li { (html::link("/charts", "Charts")) }
        }
        // div class="spacer" {}
        // footer {
        //     (html::link("/prefs", "Preferences"))
        // }
    };
    // https://www.last.fm/api/show/library.getArtists
    Ok(html)
}

#[derive(Deserialize)]
struct ApiKeyFormData {
    key: String,
    redirect_to: String,
}

#[post("/login")]
async fn login(
    form: web::Form<ApiKeyFormData>,
    pool: web::Data<SqPool>,
) -> impl Responder {
    store_api_key(&form.0.key, &pool).await.unwrap();
    redirect(&form.0.redirect_to).await
}

#[get("/artists")]
pub async fn search_artists(pool: web::Data<SqPool>) -> actix_web::Result<Markup> {
    // TODO: button for random artist (htmx?)
    // https://github.com/sekunho/emojied/blob/8b08f35ab237eb1d2417e68f92f0337fc7868c1b/src/views/url.rs#L54

    let key = get_api_key(&pool).await.map_err(error_500)?;

    let html = html! {
        (html::header())
        h2 { "Artists" }
        @if key.is_none() {
            (html::api_key_form("/artists"))
        } @else {
            form
                method="POST"
                action="/artists"
                {
                    label { "Search artist: "
                        input
                            type="text"
                            value="metallica"
                            autofocus="true"
                            name="artist" // `name` must correspond to a `Form` field
                            { }
                    button type="submit" { "Search" }
                }
            }
        }
        // button type="submit" { "Random" }
        // (PreEscaped(html::toggle()))
        // (svg())
    };

    Ok(html)
}

// For type safety reasons, html forms must always be "serialised" into a
// corresponding struct, which, upon deserialisation, then does an appropriate
// redirect
#[derive(Deserialize)]
struct ArtistFormData {
    artist: String,
}

#[post("/artists")]
async fn post_artists(form: web::Form<ArtistFormData>) -> impl Responder {
    let path = format!("/artists/{}", form.0.artist);
    redirect(&path).await
}

#[get("/artists/{artist}")]
async fn show_artist(
    // https://actix.rs/docs/url-dispatch/#scoping-routes
    // TODO: capture url params? (e.g. /artists/foo?key=val)
    path: web::Path<String>,
    pool: web::Data<SqPool>,
) -> actix_web::Result<Markup> {
    let artist = path.into_inner();

    let html = match ArtistTree::new(&artist).await {
        Ok(tree) => tree
            .build_tree(&pool)
            .await
            .map_err(error_500)?
            .as_html()
            .await
            .map_err(error_500)?,
        Err(_) => html! {
            "Artist not found: "(artist)
            p { (html::link("/", "Home")) }
        // TODO: redirect to artist search
        // https://www.last.fm/api/show/artist.search
        },
    };

    Ok(html)
}

// https://www.last.fm/api/show/geo.getTopArtists
// https://www.last.fm/api/show/user.getTopArtists

#[get("/genres")]
async fn genres() -> actix_web::Result<Markup> {
    // arguably, we don't need to cache this
    let genres = get_top_genres().await.map_err(error_500)?;
    let html = html! {
        (html::header())
        h2 { "Genres" }
        @for g in genres.0.iter() {
            (html::list_item(&html::link(&g.url, &g.name.to_lowercase()).into_string()))
        }
    // TODO: hx-swap afterend? this requires us to keep track of what page we are on
    // https://htmx.org/attributes/hx-swap/
    };
    Ok(html)
}

// note that last.fm has no concept whatsoever of "trending" with filters, aside
// from the global chart, which is next to useless for discovery
//
// https://www.last.fm/tag/rock
//
// for this purpose, bandcamp/spotify/discogs are better alternatives

// #[get("/genres/{genre}")]
// async fn show_genre(
//     path: web::Path<String>,
//     // pool: web::Data<SqPool>,
// ) -> actix_web::Result<Markup> {
//     let genre = path.into_inner();
//
//     // let html = match ArtistTree::new(&artist).await {
//     //     Ok(tree) => tree,
//     //     Err(_) => html! {
//     //         "Genre not found: "(genre)
//     //         p { (html::link("/", "Home")) }
//     //     },
//     // };
//
//     let html = html! {};
//
//     Ok(html)
// }

#[get("/charts")]
async fn get_charts() -> actix_web::Result<Markup> {
    // arguably, we don't need to cache this
    // let genres = charts::week().await.map_err(error_500)?;
    let html = html! {};
    Ok(html)
}

#[post("/youtube/{query}")]
async fn search_youtube(
    path: web::Path<String>,
    // pool: web::Data<SqPool>,
) -> actix_web::Result<Markup> {
    let query = path.into_inner();

    // yt embed would be the simplest option, but it is not very useful, unless i
    // can customise it to show only the button/progress (which was possible
    // way back in like 2009)
    // https://developers.google.com/youtube/player_parameters

    // https://developer.mozilla.org/en-US/docs/Web/HTML/Element/audio

    let html = html! {
        audio controls autoplay {
            source src=(crate::player::search_youtube(&query).await.unwrap()) {}
        }
    };

    Ok(html)
}

#[cfg(test)]
mod tests {
    use wiremock::matchers::method;
    use wiremock::matchers::path;
    use wiremock::Mock;
    use wiremock::MockServer;
    use wiremock::ResponseTemplate;

    #[tokio::test]
    async fn show_artist() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/artists/loona"))
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&mock_server)
            .await;

        reqwest::get(format!("{}/artists/loona", mock_server.uri()))
            .await
            .unwrap();
    }
}

// TODO: test api key submission (i.e. POST /login)
