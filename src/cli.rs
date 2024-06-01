use std::fs::File;
use std::io::Write;

use clap::Parser;

use crate::ArtistTree;

#[derive(Debug, Parser)]
#[command(name = "lasttree")]
struct Cli {
    #[arg(long, short)]
    artist: String,
    // TODO: format (html/dot)
}

pub fn cli() {
    let args = Cli::parse();
    let html = ArtistTree::new(&args.artist)
        // .as_dot(crate::lastfm::DotOutput::Svg)
        .as_html();

    // println!("{html}");

    let mut f = File::create("./tree.html").unwrap();
    write!(f, "{html}").unwrap();
    println!("Wrote html.tree");
}
