use actix_web::get;
use actix_web::post;
use actix_web::web;
use actix_web::HttpResponse;
use actix_web::Responder;
use maud::html;
use maud::Markup;
use serde::Deserialize;

use crate::charts::Period;
use crate::charts::User;
use crate::error_500;
use crate::get_api_key;
use crate::get_random_artist;
use crate::html;
use crate::store_api_key;
use crate::ArtistTree;
use crate::SqPool;
use crate::APP_NAME;
use crate::LASTFM_USER;

// as far as possible, this file should not contain overly complicated markup;
// simple markup is still ok for locality of behaviour

async fn redirect(path: &str) -> impl Responder {
    HttpResponse::SeeOther()
        .insert_header(("Location", path))
        .finish()
}

pub async fn not_found() -> impl Responder {
    // TODO: can we get current path?
    redirect("/").await
}

#[get("/")]
async fn home() -> actix_web::Result<Markup> {
    let html = html! {
        h1 { (APP_NAME.to_string()) }
        h2 { "Home" }
        ul {
            // note the trailing slashes!
            li { (html::link("/artists/", "Artists")) }
            li { (html::link("/charts/", "Charts")) }
            // li { (html::link("/genres", "Genres")) }
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
    store_api_key(&pool, &form.0.key).await.unwrap();
    redirect(&form.0.redirect_to).await
}

#[get("/artists/")]
pub async fn search_artists(pool: web::Data<SqPool>) -> actix_web::Result<Markup> {
    // https://github.com/sekunho/emojied/blob/8b08f35ab237eb1d2417e68f92f0337fc7868c1b/src/views/url.rs#L54

    // TODO: button for random artist (htmx?)
    let _rand = get_random_artist(&pool)
        .await
        .map_err(error_500)?
        .unwrap_or("".to_owned());

    let key = get_api_key(&pool).await.map_err(error_500)?;

    let html = html! {
        (html::header("Artists"))
        // (rand)
        @if key.is_none() {
            (html::api_key_form("/artists/"))
        } @else {
            form
                method="POST"
                action="/artists/"
                // hx-post={"/artists/"(encode(query))}
                {
                    label { "Search artist: "
                        input
                            required
                            type="text"
                            value="metallica"
                            autofocus="true"
                            // value for `name` must correspond to a `Form` field
                            name="artist"
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

#[post("/artists/")]
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

    let html = match ArtistTree::new(&artist)
        .build_tree(&pool)
        .await
        .map_err(error_500)
    {
        Ok(tree) => tree.as_html().await.map_err(error_500)?,
        Err(e) => html! {
            // "Artist not found: "(artist)
            (e)
            p { (html::link("/artists/", "Return")) }
        // TODO: try artist search, then show results in list
        // https://www.last.fm/api/show/artist.search
        },
    };

    Ok(html)
}

// https://www.last.fm/api/show/geo.getTopArtists
// https://www.last.fm/api/show/user.getTopArtists

// #[get("/genres")]
// async fn genres() -> actix_web::Result<Markup> {
//     // arguably, we don't need to cache this
//     let genres = get_top_genres().await.map_err(error_500)?;
//     let html = html! {
//         (html::header("Genres"))
//         @for g in genres.0.iter() {
//             (html::list_item(&html::link(&g.url,
// &g.name.to_lowercase()).into_string()))         }
//     // TODO: hx-swap afterend? this requires us to keep track of what page we
// are on     // https://htmx.org/attributes/hx-swap/
//     };
//     Ok(html)
// }

// note that aside from the global chart, last.fm has no concept whatsoever of
// "trending" tags with filters (e.g. top artists for tag X in the last n
// weeks), which is next to useless for discovery
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

// https://github.com/GroupTheorist12/SimpleRustWebService/blob/main/cars_service/src/main.rs#L31
#[derive(Deserialize, Debug)]
struct ChartsPath {
    user: String,
    period: Option<String>,
    // period: String,
}

/// Redirect `/charts/` -> `/charts/{default_user}`
// https://github.com/actix/actix-web/discussions/2874#discussioncomment-3647031
#[get("/charts/")]
async fn get_charts_null() -> impl Responder {
    redirect(&format!("/charts/{}/", *LASTFM_USER)).await
}

/// Redirect `/charts/{user}` -> `/charts/{user}/{period}`
#[get("/charts/{user}/")]
async fn get_charts_user(path: web::Path<ChartsPath>) -> impl Responder {
    redirect(&format!("/charts/{}/{}", path.user, Period::default())).await
}

#[get("/charts/{user}/{period}")]
async fn get_charts(
    path: web::Path<ChartsPath>,
    pool: web::Data<SqPool>,
) -> actix_web::Result<Markup> {
    let user = &path.user;

    // TODO: silently use default, or show error/warning?

    let period = match &path.period {
        Some(s) => s.as_str().try_into().unwrap_or(Period::default()),
        None => Period::default(),
    };

    // let period = path.period.as_str().try_into().unwrap_or(Period::default());

    let chart = User::new(user)
        .map_err(error_500)?
        .get_chart_period(period)
        .await
        .map_err(error_500)?;

    // println!("{:#?}", chart);
    // println!("get_charts: {}", user);

    let html = chart.as_html(user, &pool).await?;
    Ok(html)
}

#[derive(Deserialize)]
struct ChartFormData {
    user: String,
}

#[post("/charts")]
async fn post_charts(form: web::Form<ChartFormData>) -> impl Responder {
    let path = format!("/charts/{}/", form.0.user);
    redirect(&path).await
}

/// No request body is required.
#[post("/youtube/{query}")]
async fn search_youtube(path: web::Path<String>) -> actix_web::Result<Markup> {
    let query = path.into_inner();

    // yt embed would be the simplest option, but it is not very useful, unless i
    // can customise it to show only the button/progress (which was possible
    // way back in like 2009)
    // https://developers.google.com/youtube/player_parameters

    // https://developer.mozilla.org/en-US/docs/Web/HTML/Element/audio#usage_notes

    let html = match crate::player::search_youtube(&query).await {
        Ok(audio) => html! {
            p {}
            audio controls autoplay
                { source src=(audio.link) { } }
            p { (audio.title) }
        },
        Err(e) => html! {
            p {}
            p { (e) }
            p {}
            "Try searching on "
            (html::link(&format!("https://www.youtube.com/search?q={query}"), "YouTube"))
            "."
        },
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

    use crate::init_server;
    use crate::tests::TestPool;

    #[tokio::test]
    async fn show_artist() {
        let db_url = &TestPool::new(None).await.path;
        let port = 2020;
        let server = init_server(db_url, port).unwrap();

        // don't await the server, otherwise it will listen for incoming requests
        // indefinitely -- i.e., like a real server! instead, put it in a tokio thread,
        // which (somehow) terminates the server after the end of the scope
        tokio::spawn(server);

        let _ = Mock::given(method("GET"))
            .and(path("/artists/loona"))
            .respond_with(ResponseTemplate::new(200))
            .expect(1);

        // TODO: for ergonomics, we should wrap both TestPool and Server into a single
        // struct
        let addr = format!("http://localhost:{port}");

        let resp = reqwest::get(format!("{}/artists/loona", addr))
            .await
            .unwrap();

        assert_eq!(resp.status(), 200);
        assert!(resp.text().await.unwrap().contains("No API key"));
    }

    #[tokio::test]
    async fn youtube() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/youtube/metallica"))
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&mock_server)
            .await;

        let url = format!("{}/youtube/metallica", mock_server.uri());
        println!("{:?}", url);
        let resp = reqwest::Client::new().post(url).send().await.unwrap();
        assert_eq!(resp.status(), 200);
    }
}

// TODO: test api key submission (i.e. POST /login)
// TODO: test that failure to search artist (no results) redirects to /artists
// TODO: test artist with no similars -- /artists/ShyGirl
