use std::collections::HashMap;
use std::process::Command;
use std::process::Stdio;

use itertools::Itertools;
use petgraph::dot::Dot;
use petgraph::graph::Graph;
use petgraph::graph::NodeIndex;

use super::SimilarArtist;

#[derive(Debug)]
pub struct Edge(String, String, f64);

#[derive(Debug)]
/// This should be implemented as a tree, because graphs will usually produce
/// many uninteresting cycles.
pub struct ArtistTree {
    root: String,

    // edges: Vec<Edge>,
    nodes: HashMap<String, NodeIndex>,

    /// Default: 0.7
    threshold: f64,

    /// Default: 2
    depth: u8,
}

impl ArtistTree {
    /// Defaults to threshold 0.7, depth 2
    pub fn new(root: &str) -> Self {
        let root = root.to_string().to_lowercase();
        // let edges = vec![];
        let nodes = HashMap::new();
        let threshold = 0.7;
        let depth = 2;
        Self {
            root,
            // edges,
            nodes,
            threshold,
            depth,
        }
    }

    fn with_threshold(
        mut self,
        new: f64,
    ) -> Self {
        self.threshold = new;
        self
    }

    fn with_depth(
        mut self,
        new: u8,
    ) -> Self {
        self.depth = new;
        self
    }

    /// HashMap and Graph are constructed in parallel
    fn build(&mut self) -> Graph<String, String> {
        let mut graph = Graph::new();

        // TODO: refactor all lowercase calls

        let root = self.root.to_lowercase();
        let r = graph.add_node(root.clone());
        self.nodes.insert(root.clone(), r);

        for _ in 0..=self.depth {
            for parent in self.nodes.clone().keys().map(|p| p.to_lowercase()) {
                let children = match SimilarArtist::new(&parent).get_similar() {
                    Ok(ch) => ch
                        .into_iter()
                        .map(|mut a| {
                            // note: make_ascii_lowercase will leave non-ascii chars untouched
                            // a.name.make_ascii_lowercase();
                            // a
                            a.name = a.name.to_lowercase();
                            a
                        })
                        .filter(|a| a.sim_gt(0.7)),
                    Err(_) => continue,
                };
                for c in children {
                    let n1 = match self.nodes.get(&parent) {
                        Some(node) => *node,
                        None => graph.add_node(parent.to_string()),
                    };
                    let n2 = match self.nodes.get(&c.name) {
                        Some(_) => continue,
                        None => graph.add_node(c.name.clone()),
                    };
                    graph.add_edge(n1, n2, c.similarity);

                    self.nodes.insert(parent.clone(), n1);
                    self.nodes.insert(c.name, n2);
                }
            }
        }

        graph
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

    pub fn as_dot(
        &mut self,
        fmt: DotOutput,
    ) -> anyhow::Result<String> {
        // echo {out} | <fdp|dot> -Tsvg | display

        let g = &self.build();
        let dot = Dot::new(g);
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

    pub fn as_html(&mut self) -> String {
        let graph = self
            .as_dot(DotOutput::Svg)
            .unwrap()
            .lines()
            .skip(3)
            .join("\n");

        let links = self
            .nodes
            .keys()
            .map(|n| {
                format!(
                    // TODO: table
                    r#"<li><a href="https://last.fm/music/{}">{}</a></li>"#,
                    n.replace(' ', "+"),
                    n
                )
            })
            .join("\n");

        format!(
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
        )
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

    fn check_nodes(
        root: &str,
        expected_nodes: &[&str],
    ) {
        let mut tree = ArtistTree::new(root);
        tree.build();

        assert!(!tree.nodes.is_empty());

        let obtained: HashSet<&str> = tree.nodes.keys().map(|s| s.as_str()).collect();
        let expected = HashSet::from_iter(expected_nodes.iter().map(|s| s.to_owned()));
        // println!("{:#?}", tree.nodes);
        assert_eq!(obtained, expected);
    }

    #[test]
    fn test_basic() {
        check_nodes(
            "loona",
            // &["looΠΔ / odd eye circle", "loona", "loona/yyxy", "looΠΔ 1/3"],
            &["loona", "looπδ 1/3", "looπδ / odd eye circle", "loona/yyxy"],
        );
    }
}
