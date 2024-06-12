use std::collections::HashMap;
use std::fmt::Debug;
use std::fmt::Display;
use std::process::Command;
use std::process::Stdio;

use actix_web::http::header::ContentType;
use actix_web::web;
use actix_web::HttpRequest;
use actix_web::HttpResponse;
use anyhow::Context;
use itertools::Itertools;
use petgraph::dot::Dot;
use petgraph::graph::Graph;
use petgraph::graph::NodeIndex;

use crate::get_artist_from_db;
use crate::get_lastfm_similar_artists;
use crate::SqPool;

// use super::Artist;

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
// pub struct Edge(String, String, f64);
pub struct Edge {
    pub parent: String,
    pub child: String,
    pub similarity: i64,
}

#[derive(Debug)]
/// raw json -> HashMap (+ db rows) -> Graph -> Dot -> html
///
/// This should be implemented as a tree, because graphs will usually produce
/// many uninteresting cycles.
pub struct ArtistTree {
    root: String,

    // edges: Vec<Edge>,
    nodes: HashMap<String, NodeIndex>,

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
        // let edges = vec![];
        let nodes = HashMap::new();
        let threshold = 0.7;
        let depth = 2;

        let mut tree = Self {
            root,
            // edges,
            nodes,
            threshold,
            depth,
            graph: Graph::new(),
        };

        tree.build_graph(pool).await;
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
    ) {
        // let mut graph = Graph::new();

        for i in 0..=self.depth {
            let parents = match i {
                0 => {
                    // we literally only do this in order to get the canonical name via the db; the
                    // map returned by the function doesn't actually contain it!
                    get_lastfm_similar_artists(&self.root, pool).await.unwrap();
                    let root = get_artist_from_db(&self.root, pool).await.unwrap().unwrap();
                    [root].to_vec()
                }
                _ => self.nodes.clone().into_keys().collect(),
            };

            for parent in parents {
                let map = get_lastfm_similar_artists(&parent, pool).await.unwrap();

                for (c, sim) in map.iter().filter(|x| *x.1 >= 70) {
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
                }
            }

            // println!("{i} {:#?}", self.nodes);

            // panic!();
        }

        // graph
    }

    // old Vec<Edge> implementation

    // pub fn build(&mut self) {
    //     for i in 0..=self.depth {
    //         let ch = match i {
    //             0 => SimilarArtist::new(&self.root).get_edges(self.threshold),
    //             _ => {
    //                 let parents: HashSet<_> =
    //                     HashSet::from_iter(self.edges.iter().map(|e|
    // e.0.as_str()));                 let children =
    // HashSet::from_iter(self.edges.iter().map(|e| e.1.as_str()));
    //
    //                 let nodes: HashSet<_> = parents.union(&children).collect();
    //
    //                 let children = children
    //                     .difference(&parents)
    //                     .collect::<HashSet<_>>()
    //                     .iter()
    //                     .map(|p| SimilarArtist::new(p).get_edges(self.threshold))
    //                     .filter(|e| e.is_some())
    //                     .flat_map(|e| e.unwrap())
    //                     // remove cycles
    //                     .filter(|e| !nodes.contains(&e.1.as_str()))
    //                     .collect::<Vec<Edge>>();
    //                 Some(children)
    //             }
    //         };
    //         self.edges.extend(ch.unwrap());
    //     }
    // }

    // fn as_graph(&self) -> Graph<&str, f64> {
    //     // https://depth-first.com/articles/2020/02/03/graphs-in-rust-an-introduction-to-petgraph/
    //     let mut graph = Graph::new();
    //     for edge in self.edges.iter() {
    //         let Edge(parent, child, sim) = edge;
    //
    //         let n1 = match graph.node_indices().find(|i| graph[*i] == parent) {
    //             Some(node) => node,
    //             None => graph.add_node(parent.as_str()),
    //         };
    //
    //         let n2 = match graph.node_indices().find(|i| graph[*i] == child) {
    //             Some(node) => node,
    //             None => graph.add_node(child.as_str()),
    //         };
    //
    //         graph.add_edge(n1, n2, *sim);
    //     }
    //
    //     graph
    // }

    pub async fn as_dot(
        &self,
        fmt: DotOutput,
    ) -> anyhow::Result<String> {
        // echo {out} | <fdp|dot> -Tsvg | display

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

    pub async fn as_html(&self) -> anyhow::Result<String> {
        let graph = self
            .as_dot(DotOutput::Svg)
            .await?
            .lines()
            .skip(3)
            .join("\n");

        // OrderedMap::new().descending_values().into_iter();

        let links = self
            .nodes
            .keys()
            .filter(|n| **n != self.root)
            // TODO: sort by sim descending
            .map(|n| {
                format!(
                    // TODO: table
                    r#"<li><a href="https://last.fm/music/{}">{}</a></li>"#,
                    n.replace(' ', "+"),
                    n
                )
            })
            .join("\n");

        let html = format!(
            r#"
<!doctype html>
<html>
  <body>
    <h1>{}</h1>
    {}
  </body>
  <ol>
    {}
  </ol>
</html>"#,
            self.root.clone(),
            graph,
            links,
        );
        Ok(html)
    }
}

pub enum DotOutput {
    Png,
    Svg,
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::ArtistTree;
    use crate::init_test_db;

    async fn check_nodes(
        root: &str,
        expected_nodes: &[&str],
    ) {
        let pool = &init_test_db().await.pool;
        let tree = ArtistTree::new(root, pool).await;

        assert!(!tree.nodes.is_empty());

        let obtained: HashSet<&str> = tree.nodes.keys().map(|s| s.as_str()).collect();
        let expected = HashSet::from_iter(expected_nodes.iter().map(|s| s.to_owned()));
        assert_eq!(obtained, expected);

        // let html = tree.as_html().await.unwrap();
        // assert_eq!(html.matches("<li>").count(), expected.len() - 1);
    }

    #[tokio::test]
    async fn basic_tree_construction() {
        check_nodes(
            "loona",
            &["LOONA/yyxy", "LOOΠΔ 1/3", "Loona", "LOOΠΔ / ODD EYE CIRCLE"],
        )
        .await;

        check_nodes(
            "metallica",
            &[
                "Overkill",
                "Death Angel",
                "Anthrax",
                "Destruction",
                "Slayer",
                "Kreator",
                "Havok",
                "Exodus",
                "Testament",
                "Metallica",
                "Sodom",
                "Sepultura",
                "Annihilator",
                "Megadeth",
                "Pantera",
            ],
        )
        .await;
    }
}
