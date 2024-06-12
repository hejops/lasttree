use std::fmt::Debug;
use std::fmt::Display;
use std::process::Command;
use std::process::Stdio;

use actix_web::http::header::ContentType;
use actix_web::web;
use actix_web::HttpRequest;
use actix_web::HttpResponse;
use anyhow::Context;
use indexmap::IndexMap;
use petgraph::dot::Dot;
use petgraph::graph::Graph;
use petgraph::graph::NodeIndex;

use crate::get_canonical_name;
use crate::get_similar_artists;
use crate::SqPool;

/// Convert arbitrary error types to `actix_web::Error` with HTTP 500
pub fn error_500<T>(e: T) -> actix_web::Error
where
    T: Debug + Display + 'static,
{
    actix_web::error::ErrorInternalServerError(e)
}

pub async fn home() -> Result<HttpResponse, actix_web::Error> {
    let html = "hello world";
    Ok(HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(html))
}

/// `GET /artists`
pub async fn search_artist() -> Result<HttpResponse, actix_web::Error> {
    let html = "search artist:";
    Ok(HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(html))
}

/// `GET /artists/{artist}`
pub async fn get_artist(
    req: HttpRequest,
    pool: web::Data<SqPool>,
) -> Result<HttpResponse, actix_web::Error> {
    let artist = req
        .match_info()
        .get("artist")
        .context("no artist supplied")
        .map_err(error_500)?;

    let tree = ArtistTree::new(artist, &pool).await;
    let html = tree.as_html().await.map_err(error_500)?;

    Ok(HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(html))
}

#[derive(Debug)]
/// raw json -> IndexMap (+ db rows) -> Graph -> Dot -> html
///
/// This should be implemented as a tree, because graphs will usually produce
/// many uninteresting cycles.
pub struct ArtistTree {
    pub root: String,

    // edges: Vec<Edge>,
    pub nodes: IndexMap<String, NodeIndex>,

    #[allow(dead_code)]
    /// Default: 0.7
    threshold: f64,

    #[allow(dead_code)]
    /// Default: 2
    depth: u8,

    graph: Graph<String, String>,
}

impl ArtistTree {
    /// Defaults to `threshold` 0.7, `depth` 2
    pub async fn new(
        root: &str,
        pool: &SqPool,
    ) -> Self {
        let root = root.to_string();
        let nodes = IndexMap::new();
        let threshold = 0.7;
        let depth = 2;

        let mut tree = Self {
            root,
            nodes,
            threshold,
            depth,
            graph: Graph::new(),
        };

        tree.build_graph(pool).await.unwrap();
        tree
    }

    // fn with_threshold(
    //     mut self,
    //     new: f64,
    // ) -> Self {
    //     self.threshold = new;
    //     self
    // }
    //
    // fn with_depth(
    //     mut self,
    //     new: u8,
    // ) -> Self {
    //     self.depth = new;
    //     self
    // }

    /// `self.nodes` is only used to keep track of what has been added to the
    /// `Graph`. It is not otherwise used.
    async fn build_graph(
        &mut self,
        pool: &SqPool,
    ) -> anyhow::Result<()> {
        for i in 0..=self.depth {
            let parents = match i {
                0 => {
                    // we literally only do this in order to store the canonical name in the db and
                    // get it back; the map returned by the function doesn't actually contain it!
                    get_similar_artists(&self.root, pool).await?;
                    let root = get_canonical_name(&self.root, pool).await?.context("")?;
                    self.root = root.clone(); // override with the canonical
                    println!("{:#?}", self);
                    [root].to_vec()
                }
                _ => self.nodes.clone().into_keys().collect(),
            };

            for parent in parents {
                let map = get_similar_artists(&parent, pool).await?;

                // println!("{}", parent);
                // for (k, v) in map.iter().take(5) {
                //     println!("{k} {v}");
                // }

                for (c, sim) in map.iter().filter(|x| *x.1 >= 70) {
                    // if c.is_empty() {
                    //     continue;
                    // }
                    let n1 = match self.nodes.get(&parent) {
                        Some(node) => *node,
                        None => self.graph.add_node(parent.clone()),
                    };
                    let n2 = match self.nodes.get(c) {
                        Some(_) => continue,
                        None => self.graph.add_node(c.to_string()),
                    };
                    self.graph.add_edge(n1, n2, sim.to_string());

                    self.nodes.insert(parent.clone(), n1);
                    self.nodes.insert(c.to_string(), n2);
                    // println!("new node: {c} {n2:?}");
                }
                // println!("{:?}", self.nodes);
            }

            // println!("{i} {:#?}", self.nodes);

            // panic!();
        }

        // graph
        Ok(())
    }

    pub async fn as_dot(
        &self,
        fmt: DotOutput,
    ) -> anyhow::Result<String> {
        // echo {out} | <fdp|dot> -Tsvg | display

        // println!("starting dot {:#?}", self.graph);
        let dot = Dot::new(&self.graph);
        let ext = match fmt {
            DotOutput::Png => "png",
            DotOutput::Svg => "svg",
        };

        // let out = format!("{}.{}", self.root, ext);

        let echo = Command::new("echo")
            .arg(dot.to_string())
            .stdout(Stdio::piped())
            .spawn()?;
        let _fdp = Command::new("dot")
            .args(["-T", ext])
            .stdin(Stdio::from(echo.stdout.unwrap()))
            // .stdout(Stdio::piped())
            // .args(["-o", &out])
            .output()?
            .stdout;

        // https://stackoverflow.com/a/42993724
        Ok(String::from_utf8_lossy(&_fdp).to_string())

        // Ok(_fdp)

        // Command::new("display")
        //     // .stdin(Stdio::from(fdp.stdout.unwrap()))
        //     .arg(out)
        //     .spawn()?
        //     .wait()?;

        // Ok(())
    }
}

pub enum DotOutput {
    Png,
    Svg,
}

#[cfg(test)]
mod tests {

    use super::ArtistTree;
    use crate::init_test_db;

    // TODO: initial graph layout often different from when cached data is
    // available. this suggests that we should cache everything first before
    // constructing graph (or something to that effect)

    async fn check_nodes(
        root: &str,
        expected_nodes: &[&str],
    ) {
        let pool = &init_test_db().await.pool;
        let tree = ArtistTree::new(root, pool).await;

        assert!(!tree.nodes.is_empty());

        let obtained_nodes: Vec<&str> = tree.nodes.keys().map(|s| s.as_str()).collect();
        // println!("nodes {:#?}", tree.nodes);
        // println!("nodes vec {:#?}", obtained_nodes);
        assert_eq!(obtained_nodes, expected_nodes);

        let html = tree.as_html().await.unwrap();
        assert_eq!(
            html.matches(
                // "<li>"
                "<tr><td>"
            )
            .count(),
            expected_nodes.len() - 1
        );
    }

    #[tokio::test]
    async fn node_order() {
        check_nodes(
            "loona",
            &["Loona", "LOOΠΔ 1/3", "LOONA/yyxy", "LOOΠΔ / ODD EYE CIRCLE"],
        )
        .await;

        // // harder to test node order in larger graphs
        // check_nodes(
        //     "metallica",
        //     &[
        //         "Metallica",
        //         "Megadeth",
        //         "Exodus",
        //         "Anthrax",
        //         "Slayer",
        //         "Testament",
        //         "Death Angel",
        //         "Overkill",
        //         "Kreator",
        //         "Destruction",
        //         "Havok",
        //         "Sodom",
        //         "Annihilator",
        //         "Pantera",
        //         "Sepultura",
        //     ],
        // )
        // .await;
    }
}
