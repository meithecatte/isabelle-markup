use argh::FromArgs;
use std::path::PathBuf;
use yxml::Node;

#[derive(FromArgs)]
/// Convert output of 'isabelle dump' to HTML.
struct Options {
    #[argh(positional)]
    /// path to dump
    dump_path: PathBuf,
}

fn print_text(node: &Node<'_>) {
    match node {
        Node::Text(t) => print!("{}", t),
        Node::Tag { children, .. } => children.iter().for_each(print_text),
    }
}

fn main() {
    let options: Options = argh::from_env();
    let yxml = std::fs::read_to_string(options.dump_path).unwrap();
    let nodes = yxml::parse(&yxml).unwrap();
    nodes.iter().for_each(print_text);
}
