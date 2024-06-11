use std::env;

use actix_web::web;
use actix_web::App;
use actix_web::HttpServer;
use lasttree::get_artist;
use lasttree::init_db;

#[tokio::main] // requires tokio features: macros, rt-multi-thread
async fn main() -> anyhow::Result<()> {
    // let addr = format!("{}:{}", "127.0.0.1", 3838);
    // let listener = TcpListener::bind(addr)?;

    // https://github.com/actix/examples/blob/6334049545e0a03888b4dc57a9d447e0292164ee/databases/sqlite/src/main.rs#L51

    let db_url = env::var("DATABASE_URL").unwrap_or("sqlite://lasttree.db".to_owned());
    let pool = web::Data::new(init_db(&db_url)?);

    HttpServer::new(move || {
        App::new()
            // .route("/", web::get().to(home))
            .route("/artist/{artist}", web::get().to(get_artist))
            .app_data(pool.clone())
    })
    .bind(("127.0.0.1", 3838))?
    .run()
    .await?;

    Ok(())
}
