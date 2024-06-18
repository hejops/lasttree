use std::env;

use actix_web::web;
use actix_web::App;
use actix_web::HttpServer;
use lasttree::init_db;
use lasttree::routes;

#[tokio::main] // requires tokio features: macros, rt-multi-thread
async fn main() -> anyhow::Result<()> {
    // let addr = format!("{}:{}", "127.0.0.1", 3838);
    // let listener = TcpListener::bind(addr)?;

    // https://github.com/actix/examples/blob/6334049545e0a03888b4dc57a9d447e0292164ee/databases/sqlite/src/main.rs#L51

    let db_url = env::var("DATABASE_URL").unwrap_or("sqlite://lasttree.db".to_owned());
    let pool = web::Data::new(init_db(&db_url)?);

    HttpServer::new(move || {
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
            .service(routes::login)
            .service(routes::search_artists)
            .service(routes::post_artists)
            .service(routes::show_artist)
            .service(routes::genres)
            .service(routes::search_youtube)
            .default_service(web::route().to(routes::not_found))
            .app_data(pool.clone())
    })
    .bind(("127.0.0.1", 3838))?
    .run()
    .await?;

    Ok(())
}
