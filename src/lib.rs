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

// on `pub mod` vs `mod + pub use`:
//
// my principle is to use `pub use foo::*;` if it is ok to refer to (/import)
// functions "directly", without importing their namespace. this usually means
// functions precisely named.
//
// https://users.rust-lang.org/t/principles-for-using-mod-vs-pub-mod/27814/2

mod artists;
mod db;
pub mod dot;
mod genres;
pub mod html;
pub mod routes;
pub mod tests;
mod tree;
pub use db::*;
pub use genres::*;
pub use tree::*;

lazy_static::lazy_static! {
    /// Used only for testing
    static ref LASTFM_KEY: String =
        std::env::var("LASTFM_KEY").expect("Environment variable $LASTFM_KEY must be set");
    static ref APP_NAME: String = "Last".to_string();

    /// A base64 engine used to crudely pass data from one endpoint to another
    static ref BASE64: base64::engine::GeneralPurpose = base64::engine::GeneralPurpose::new(
        &base64::alphabet::URL_SAFE,
        base64::engine::general_purpose::NO_PAD
    );
}
