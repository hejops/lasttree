use actix_web::get;
use actix_web::http::header::ContentType;
use actix_web::web;
use actix_web::HttpRequest;
use actix_web::HttpResponse;
use anyhow::Context;
use maud::Markup;

use crate::error_500;
use crate::ArtistTree;
use crate::SqPool;

#[get("/")]
pub async fn home() -> Result<HttpResponse, actix_web::Error> {
    let html = "hello world";
    Ok(HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(html))
}

#[get("/artists")]
pub async fn search_artist() -> Result<HttpResponse, actix_web::Error> {
    let html = "search artist:";
    Ok(HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(html))
}

#[get("/artists/{artist}")]
pub async fn get_artist(
    req: HttpRequest,
    pool: web::Data<SqPool>,
) -> actix_web::Result<Markup> {
    let artist = req
        .match_info()
        .get("artist")
        .context("no artist supplied")
        .map_err(error_500)?;

    let tree = ArtistTree::new(artist, &pool).await;
    let html = tree.as_html().await.map_err(error_500)?;

    Ok(html)
}
