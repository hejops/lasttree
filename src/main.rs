use std::env;

use lasttree::init_server;

#[tokio::main] // requires tokio features: macros, rt-multi-thread
async fn main() -> anyhow::Result<()> {
    let db_url = env::var("DATABASE_URL").unwrap_or("sqlite://lasttree.db".to_owned());
    let port = 3838;
    init_server(&db_url, port)?.await?;

    Ok(())
}
