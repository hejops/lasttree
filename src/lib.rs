//! Written as an exercise to:
//!
//! 1. implement basic tree structures,
//! 2. using data from the Last.fm API,
//! 3. for the purpose of music discovery

// the graph was initially implemented as a naive Vec<String> + Vec<(usize,
// usize)> -- see https://github.com/eliben/code-for-blog/blob/master/2021/rust-bst/src/nodehandle.rs.
// eventually i found Vec indexing really annoying, and switched over to an
// "edge-only" Vec<Edge>. then i also found -that- ugly, and switched to a
// HashMap.

use std::env;

use lazy_static::lazy_static;

// on `pub mod` vs `mod + pub use`:
//
// my principle is to use `pub use foo::*;` if it is ok to refer to (/import)
// functions "directly", without importing their namespace. this usually means
// functions precisely named.
//
// https://users.rust-lang.org/t/principles-for-using-mod-vs-pub-mod/27814/2

mod db;
pub mod html;
mod lastfm;
pub mod routes;
pub mod tests;
mod tree;
pub use db::*;
// pub use html::*;
// pub use lastfm::*;
pub use tree::*;

lazy_static! {
    static ref LASTFM_KEY: String =
        env::var("LASTFM_KEY").expect("Environment variable $LASTFM_KEY must be set");
    static ref APP_NAME: String = "Last".to_string();
}
