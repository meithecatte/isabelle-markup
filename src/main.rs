use argh::FromArgs;
use std::path::PathBuf;
use yxml::Node;
use std::io;

mod output;

use output::{Tag, HTMLOutput};

#[derive(FromArgs)]
/// Convert output of 'isabelle dump' to HTML.
struct Options {
    #[argh(positional)]
    /// path to dump
    dump_path: PathBuf,

    #[argh(positional)]
    /// output path
    out_path: PathBuf,
}

fn write_node(output: &mut HTMLOutput, node: &Node<'_>) -> io::Result<()> {
    match node {
        Node::Text(t) => {
            output.write_text(t)?;
        }
        Node::Tag { name, attrs, children } => {
            let close_tag = match *name {
                // Ignore xml_body for now - this tag is part of the mechanism that
                // provides type information on hover.
                "xml_body" => return Ok(()),
                "keyword1" | "keyword2" | "keyword3" => {
                    let mut classes = name.to_string();
                    if let Some(kind) = attrs.get("kind") {
                        classes.push(' ');
                        classes.push_str(kind);
                    }
                    output.open_tag(Tag::SpanClass(classes))?;
                    true
                }
                "comment" => {
                    output.open_tag(Tag::SpanClass(name.to_string()))?;
                    true
                }
                _ => false,
            };

            for child in children {
                write_node(output, child)?;
            }

            if close_tag {
                output.close_tag()?;
            }
        }
    }

    Ok(())
}

fn main() {
    let options: Options = argh::from_env();
    let yxml = std::fs::read_to_string(&options.dump_path).unwrap();
    let nodes = yxml::parse(&yxml).unwrap();

    let mut output = HTMLOutput::to_file(&options.out_path).unwrap();
    for node in &nodes {
        write_node(&mut output, &node).unwrap();
    }
}
