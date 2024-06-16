use std::fmt::Debug;
use std::fmt::Display;

use anyhow::Context;
use indexmap::IndexMap;
use petgraph::algo::astar;
use petgraph::graph::Graph;
use petgraph::graph::NodeIndex;
use petgraph::visit::NodeIndexable;

use crate::SqPool;

/// Convert arbitrary error types to `actix_web::Error` with HTTP 500. Note
/// that, as the type signature suggests, `T` must implement `Display`.
pub fn error_500<T>(e: T) -> actix_web::Error
where
    T: Debug + Display + 'static,
{
    actix_web::error::ErrorInternalServerError(e)
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

    pub graph: Graph<String, i64>,
}

impl ArtistTree {
    /// Defaults to `threshold` 0.7, `depth` 2
    pub async fn new(root: &str) -> anyhow::Result<Self> {
        let root = root.to_string();
        let nodes = IndexMap::new();
        let threshold = 0.7;
        let depth = 2;

        // let mut tree = Self {
        let tree = Self {
            root,
            nodes,
            threshold,
            depth,
            graph: Graph::new(),
        };

        // tree.build_graph(pool).await?;
        Ok(tree)
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
    ///
    /// Note: `self.root` will be replaced with the canonical name.
    pub async fn build_tree(
        mut self,
        pool: &SqPool,
        // ) -> anyhow::Result<()> {
    ) -> anyhow::Result<Self> {
        for i in 0..=self.depth {
            let parents = match i {
                0 => {
                    // we literally only do this in order to store the canonical name in the db and
                    // get it back; the map returned by the function doesn't actually contain it!
                    self.get_similar_artists(pool).await?;
                    let canon = self.canonical_name(pool).await?.context("")?;
                    // println!("{:?}", canon);
                    self.root = canon.clone(); // override with the canonical

                    // println!("{:#?}", self);
                    [canon].to_vec()
                }
                _ => self.nodes.clone().into_keys().collect(),
            };

            for parent in parents {
                let map = ArtistTree::new(&parent)
                    .await?
                    .get_similar_artists(pool)
                    .await?;

                for (c, sim) in map.iter().filter(|x| *x.1 >= 70) {
                    let n1 = match self.nodes.get(&parent) {
                        Some(node) => *node,
                        None => self.graph.add_node(parent.clone()),
                    };
                    let n2 = match self.nodes.get(c) {
                        Some(_) => continue,
                        None => self.graph.add_node(c.to_string()),
                    };
                    self.graph.add_edge(n1, n2, *sim);

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
        Ok(self)
    }

    fn get_node_index(
        &self,
        node_label: &str,
    ) -> Option<NodeIndex> {
        self.graph
            .node_indices()
            .find(|i| self.graph[*i] == node_label)
    }

    /// Given an arbitrary `child` node, calculate its similarity to the root
    /// node by multiplying edge weights successively
    pub fn get_child_similarity(
        &self,
        child: &str,
    ) -> i64 {
        // println!("{:#?}", self.graph);
        let root = self.graph.from_index(0);
        let target = self.get_node_index(child).unwrap();

        // println!("0 {:#?}", self.graph.node_weight(root)); // n_idx -> Option<str>

        // generally, Graph methods only operate on single edges. to get a path between
        // 2 arbitrary edges, an `algorithm` is required

        // https://docs.rs/petgraph/latest/petgraph/algo/astar/fn.astar.html#example
        let path = astar(
            //
            &self.graph,
            root,
            |n| n == target,
            |_| 1,
            |_| 1,
        )
        .unwrap()
        .1;

        // let x = self.graph.index_twice_mut(path[0], path[3]);
        // println!("{:?}", x);

        // https://github.com/a-b-street/abstreet/blob/35d669cf7aa9b6d24cd0cfe423f0dfc4037b4357/map_model/src/map.rs#L880
        path.windows(2)
            .map(|pair| self.graph.find_edge(pair[0], pair[1]).unwrap())
            .map(|e| self.graph.edge_weight(e).unwrap())
            .fold(100, |acc, x| (acc * x) / 100)
    }
}

#[cfg(test)]
mod tests {

    use super::ArtistTree;
    use crate::tests::TestPool;

    // TODO: initial graph layout often different from when cached data is
    // available. this suggests that we should cache everything first before
    // constructing graph (or something to that effect)

    async fn check_nodes(
        root: &str,
        expected_nodes: &[&str],
    ) {
        let pool = &TestPool::new().await.with_key().await.pool;
        let tree = ArtistTree::new(root)
            .await
            .unwrap()
            .build_tree(pool)
            .await
            .unwrap();

        assert!(!tree.nodes.is_empty());

        let obtained_nodes: Vec<&str> = tree.nodes.keys().map(|s| s.as_str()).collect();
        // println!("nodes {:#?}", tree.nodes);
        // println!("nodes vec {:#?}", obtained_nodes);
        assert_eq!(obtained_nodes, expected_nodes, "nodes do not match");

        let html = tree.as_html().await.unwrap().clone().into_string();

        assert_eq!(html.matches("<tr><td>").count(), expected_nodes.len() - 1);

        assert_eq!(
            html.matches(r#"href="/artists/"#).count(),
            // graph has n links, table has n - 1 links
            expected_nodes.len() * 2 - 1,
            "{root}"
        );

        assert_eq!(
            html.matches(r#"xlink:href="/artists/"#).count(),
            // svg links are "a xlink:href", not "a href"
            expected_nodes.len(),
            "{root}"
        );
    }

    #[tokio::test]
    async fn node_order() {
        check_nodes(
            "loona",
            // artist (canonical), followed by similar artists in descending similarity
            &["Loona", "LOOΠΔ 1/3", "LOONA/yyxy", "LOOΠΔ / ODD EYE CIRCLE"],
        )
        .await;
    }

    #[tokio::test]
    async fn child_similarity() {
        let pool = &TestPool::new().await.with_key().await.pool;
        let tree = ArtistTree::new("metallica")
            .await
            .unwrap()
            .build_tree(pool)
            .await
            .unwrap();
        let sim = tree.get_child_similarity("Annihilator");
        assert_eq!(sim, 52); // TODO: this number is not fixed
    }
}
