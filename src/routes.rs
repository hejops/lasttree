use actix_web::get;
use actix_web::web;
use maud::html;
use maud::Markup;

use crate::error_500;
use crate::ArtistTree;
use crate::SqPool;

#[get("/")]
pub async fn home() -> actix_web::Result<Markup> {
    let html = html! {"hello world"};
    Ok(html)
}

#[get("/artists")]
pub async fn search_artist() -> actix_web::Result<Markup> {
    let html = html! {"search artist:"};
    Ok(html)
}

#[get("/artists/{artist}")]
pub async fn get_artist(
    // https://actix.rs/docs/url-dispatch/#scoping-routes
    path: web::Path<String>,
    pool: web::Data<SqPool>,
) -> actix_web::Result<Markup> {
    let artist = path.into_inner();

    let tree = ArtistTree::new(&artist, &pool).await;

    let html = tree.as_html().await.map_err(error_500)?;
    Ok(html)
}
