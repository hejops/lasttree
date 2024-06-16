use actix_web::get;
use actix_web::post;
use actix_web::web;
use actix_web::HttpResponse;
use actix_web::Responder;
use maud::html;
use maud::Markup;
use serde::Deserialize;

use crate::error_500;
use crate::html; // should not conflict with maud::html
use crate::ArtistTree;
use crate::SqPool;
use crate::APP_NAME;

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
        ul {
            // fill this up once several endpoints are "ready"
            li { a href=("/artists") { "Artists" } }
        }
    };
    Ok(html)
}

#[get("/artists")]
pub async fn search_artists() -> actix_web::Result<Markup> {
    // TODO: button for random artist (htmx?)
    // https://github.com/sekunho/emojied/blob/8b08f35ab237eb1d2417e68f92f0337fc7868c1b/src/views/url.rs#L54
    let html = html! {
        a href=("/") { "Home" }
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
        },
    };

    Ok(html)
}

// #[derive(Deserialize)]
// struct ArtistTreeData {
//     svg: String,
// }
//
// #[post("/artists/{artist}/svg")]
// async fn post_artists_svg(form: web::Form<ArtistTreeData>) -> impl Responder
// {     println!("received POST /artists/X/svg");
//     assert!(form.0.svg.ends_with("</svg>"));
//     let path = format!("/svg/{}", BASE64.encode(form.0.svg));
//     redirect(&path).await
// }
//
// // TODO: should probably be scoped?
// // TODO: redirect to / on decode failure
// #[get("/svg/{svg}")]
// async fn show_artist_svg(path: web::Path<String>) ->
// actix_web::Result<HttpResponse> {     let b64 = path.into_inner();
//     // let byt = match BASE64.decode(b64) {
//     //     Ok(b) => b,
//     //     Err(_) => return redirect("/").await,
//     // };
//     let byt = BASE64.decode(b64).map_err(error_500)?;
//     let svg = std::str::from_utf8(&byt)?.to_string();
//     assert!(svg.ends_with("</svg>"));
//
//     // Ok(html! { (PreEscaped(svg))})
//     Ok(HttpResponse::Ok()
//         .content_type(ContentType::html())
//         .body(svg))
// }

#[cfg(test)]
mod tests {
    use wiremock::matchers::method;
    use wiremock::matchers::path;
    use wiremock::Mock;
    use wiremock::MockServer;
    use wiremock::ResponseTemplate;

    #[tokio::test]
    async fn get_artist() {
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
