use std::fs;
use std::path::Path;

use uuid::Uuid;

use crate::init_db;
use crate::store_api_key;
use crate::SqPool;
use crate::LASTFM_KEY;

pub struct TestPool {
    pub pool: SqPool,
    pub path: String,
}

/// custom `Drop` avoids clogging up your whatever dir when running lots of
/// tests
impl Drop for TestPool {
    fn drop(&mut self) { fs::remove_file(&self.path).unwrap(); }
}

impl TestPool {
    pub async fn new() -> Self {
        let id = Uuid::new_v4();
        let path = format!("/tmp/test-{id}.db");
        // let path = format!("test-{id}.db");
        if Path::new(&path).exists() {
            fs::remove_file(&path).unwrap();
        }
        let pool = init_db(&format!("sqlite://{path}")).unwrap();
        sqlx::migrate!().run(&pool).await.unwrap();

        TestPool { pool, path }
    }

    pub async fn with_key(self) -> Self {
        store_api_key(&LASTFM_KEY, &self.pool).await.unwrap();
        self
    }
}
