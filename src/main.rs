use argh::FromArgs;
use std::io;
use std::path::PathBuf;
use yxml::Node;

mod output;
mod symbols;

use output::{HTMLOutput, Tag};

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

fn write_node(output: &mut HTMLOutput<'_>, node: &Node<'_>) -> io::Result<()> {
    match node {
        Node::Text(t) => {
            output.write_text(t)?;
        }
        Node::Tag {
            name,
            attrs,
            children,
        } => {
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
                "binding" | "tfree" | "tvar" | "free" | "skolem" | "bound" | "var" | "literal"
                | "inner_numeral" | "inner_quoted" | "inner_cartouche" | "inner_string"
                | "antiquoted" | "comment1" | "comment2" | "comment3" | "dynamic_fact"
                | "quasi_keyword" | "operator" | "string" | "alt_string" | "verbatim"
                | "cartouche" | "comment" | "improper" | "antiquote" | "raw_text"
                | "plain_text" => {
                    output.open_tag(Tag::SpanClass(name.to_string()))?;
                    true
                }
                _ => false,
            };

            let tooltip = match *name {
                "citation" => Some("citation"),
                "token_range" => Some("inner syntax token"),
                "free" => Some("free variable"),
                "skolem" => Some("skolem variable"),
                "bound" => Some("bound variable"),
                "var" => Some("schematic variable"),
                "tfree" => Some("free type variable"),
                "tvar" => Some("schematic type variable"),
                _ => None,
            };

            if let Some(s) = tooltip {
                output.tooltip_html(s);
            }

            for child in children {
                write_node(output, child)?;
            }

            if tooltip.is_some() {
                output.tooltip_end();
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
