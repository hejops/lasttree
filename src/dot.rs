use graphviz_rust::attributes::EdgeAttributes;
use graphviz_rust::attributes::NodeAttributes;
use graphviz_rust::dot_generator::*;
use graphviz_rust::dot_structures::*;
use graphviz_rust::exec_dot;
use graphviz_rust::printer::DotPrinter;
use graphviz_rust::printer::PrinterContext;
use petgraph::visit::EdgeRef;
use petgraph::visit::IntoNodeReferences;

use crate::ArtistTree;

fn quote(s: &str) -> String { format!("{:?}", s) }

impl ArtistTree {
    // https://github.com/egraphs-good/egraph-serialize/blob/5838c036623e91540831745b1574539e01c8cb23/src/graphviz.rs#L36
    pub fn as_dot(&self) -> graphviz_rust::dot_structures::Graph {
        let mut g = graph!(di id!(), vec![]);

        for n in self.graph.node_references() {
            let url = format!("/artists/{}", n.1); // encode?
            let node = node!(
                n.0.index();
                NodeAttributes::label(quote(n.1)),
                NodeAttributes::URL(quote(&url))
            );
            g.add_stmt(stmt!(node));
        }

        for e in self.graph.edge_references() {
            let src = e.source().index();
            let trg = e.target().index();
            let edge = edge!(node_id!(src) => node_id!(trg);
                EdgeAttributes::label(quote(&e.weight().to_string()))
            );
            g.add_stmt(stmt!(edge));
        }

        g
    }

    pub fn as_svg(&self) -> String {
        let dot_str = self.as_dot().print(&mut PrinterContext::default());
        let args = vec![graphviz_rust::cmd::Format::Svg.into()];
        let byt = exec_dot(dot_str, args).unwrap();
        String::from_utf8(byt).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use graphviz_rust::printer::DotPrinter;
    use graphviz_rust::printer::PrinterContext;

    use crate::tests::init_test_db;
    use crate::ArtistTree;

    #[tokio::test]
    async fn simple() {
        let pool = &init_test_db().await.pool;
        let a = ArtistTree::new("loona")
            .await
            .unwrap()
            .build_tree(pool)
            .await
            .unwrap();

        let dot = a.as_dot().print(&mut PrinterContext::default());
        assert_eq!(
            dot,
            "\
digraph  {
  0[label=\"Loona\",URL=\"/artists/Loona\"]
  1[label=\"LOOΠΔ 1/3\",URL=\"/artists/LOOΠΔ 1/3\"]
  2[label=\"LOONA/yyxy\",URL=\"/artists/LOONA/yyxy\"]
  3[label=\"LOOΠΔ / ODD EYE CIRCLE\",URL=\"/artists/LOOΠΔ / ODD EYE CIRCLE\"]
  0 -> 1 [label=\"100\"]
  0 -> 2 [label=\"97\"]
  0 -> 3 [label=\"86\"]
}"
        );
    }
}
