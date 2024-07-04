//! Written as an exercise to:
//!
//! 1. implement basic tree structures,
//! 2. using data from the Last.fm API,
//! 3. for the purpose of music discovery

// on `pub mod` vs `mod + pub use`:
//
// my principle is to use `pub use foo::*;` if it is ok to refer to (/import)
// functions "directly", without importing their namespace. this usually means
// functions precisely named.
//
// https://users.rust-lang.org/t/principles-for-using-mod-vs-pub-mod/27814/2

mod artists;
pub mod charts;
mod db;
pub mod dot;
mod genres;
pub mod html;
mod player;
pub mod routes;
pub mod tests;
mod tree;
pub mod utils;
pub use db::*;
pub use genres::*;
pub use tree::*;

lazy_static::lazy_static! {
    static ref WEEK: u32 = 60 * 60 * 24 * 7;

    /// Used only for testing
    static ref LASTFM_USER: String =
        std::env::var("LASTFM_USER").expect("Environment variable $LASTFM_USER must be set");
    static ref LASTFM_KEY: String =
        std::env::var("LASTFM_KEY").expect("Environment variable $LASTFM_KEY must be set");

    static ref LASTFM_URL: String = "http://ws.audioscrobbler.com/2.0/?format=json".to_string();

    static ref APP_NAME: String = "Last".to_string();

    /// A base64 engine used to crudely pass data from one endpoint to another
    static ref BASE64: base64::engine::GeneralPurpose = base64::engine::GeneralPurpose::new(
        &base64::alphabet::URL_SAFE,
        base64::engine::general_purpose::NO_PAD
    );
}

/// To start the server:
/// ```no_run
/// use lasttree::init_server;
///
/// # tokio_test::block_on(async {
/// let server = init_server("db_url", 7777).unwrap();
/// server.await.unwrap(); // in production
///
/// let server = init_server("db_url", 7777).unwrap();
/// tokio::spawn(server); // in test
/// # })
/// ```
pub fn init_server(
    db_url: &str,
    port: u16,
) -> anyhow::Result<actix_web::dev::Server> {
    // use actix_web::dev::Server;
    use actix_web::web;
    use actix_web::App;
    use actix_web::HttpServer;

    // let addr = format!("{}:{}", "127.0.0.1", 3838);
    // let listener = TcpListener::bind(addr)?;

    // https://github.com/actix/examples/blob/6334049545e0a03888b4dc57a9d447e0292164ee/databases/sqlite/src/main.rs#L51

    let pool = web::Data::new(init_db(db_url)?);

    let server = HttpServer::new(move || {
        App::new()
            // i prefer
            //      .service(foo) + #[get("/foo")] fn foo
            // over
            //      .route("/foo", web::get().to(foo))
            // because it keeps `App` clean, and the route is more closely coupled to the function
            // https://actix.rs/docs/url-dispatch/#scoping-routes
            //
            // .route("/", web::get().to(home))
            .service(routes::home)
            .service(routes::search_artists)
            .service(routes::post_artists)
            .service(routes::show_artist)
            // .service(routes::genres)
            .service(routes::get_charts)
            .service(routes::get_charts_user)
            .service(routes::get_charts_null)
            // TODO: how to get compiler to remind me to use POST routes?
            .service(routes::post_charts)
            // .service(web::resource(["/charts/", "/charts/{user}"]).to(routes::get_charts))
            // .service(web::resource("/charts/").to(routes::get_charts))
            // auxiliary
            .service(routes::login)
            .service(routes::search_youtube)
            .default_service(web::route().to(routes::not_found))
            .app_data(pool.clone())
    })
    .bind(("127.0.0.1", port))?
    .run();
    Ok(server)
}
