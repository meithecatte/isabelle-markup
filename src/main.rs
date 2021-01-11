use argh::FromArgs;
use std::fs::File;
use std::io::{self, prelude::*, BufWriter};
use std::path::PathBuf;
use yxml::Node;

mod ir;
mod symbols;

use ir::*;

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

fn processed_ir<'a>(input: &[Node<'a>]) -> Vec<TagTree<'a>> {
    let mut ir: Vec<TagTree> = input.iter().flat_map(lower_node).collect();
    trim_empty(&mut ir);
    merge_tooltips(&mut ir, None);
    ir
}

fn render_to_string(input: &[Node<'_>]) -> String {
    let ir = processed_ir(input);
    let mut buf = Vec::new();
    write_nodes(&mut io::Cursor::new(&mut buf), &ir, false).unwrap();
    String::from_utf8(buf).unwrap()
}

fn lower_node<'input>(node: &Node<'input>) -> Vec<TagTree<'input>> {
    match node {
        Node::Text(s) => vec![TagTree::Text(s)],
        Node::Tag {
            name,
            attrs,
            children,
        } => {
            let class = match *name {
                // Ignore xml_body for now - this tag is part of the mechanism that
                // provides type information on hover.
                "xml_body" => return vec![],
                "keyword1" | "keyword2" | "keyword3" => {
                    let mut classes = name.to_string();
                    if let Some(kind) = attrs.get("kind") {
                        classes.push(' ');
                        classes.push_str(kind);
                    }
                    Some(classes)
                }
                "binding" | "tfree" | "tvar" | "free" | "skolem" | "bound" | "var"
                | "literal" | "inner_numeral" | "inner_quoted" | "inner_cartouche"
                | "inner_string" | "antiquoted" | "comment1" | "comment2"
                | "comment3" | "dynamic_fact" | "quasi_keyword" | "operator"
                | "string" | "alt_string" | "verbatim" | "cartouche" | "comment"
                | "improper" | "antiquote" | "raw_text" | "plain_text" => {
                    Some(name.to_string())
                }
                _ => None,
            };

            let tooltip = match *name {
                "citation" => Some("citation".to_owned()),
                "token_range" => Some("inner syntax token".to_owned()),
                "free" => Some("free variable".to_owned()),
                "skolem" => Some("skolem variable".to_owned()),
                "bound" => Some("bound variable".to_owned()),
                "var" => Some("schematic variable".to_owned()),
                "tfree" => Some("free type variable".to_owned()),
                "tvar" => Some("schematic type variable".to_owned()),
                "xml_elem" => {
                    let prefix = match attrs["xml_name"] {
                        "ML_typing" => "ML: ",
                        "typing" | "sorting" => ":: ",
                        "class_parameter" => "",
                        name => unimplemented!("{}", name),
                    };

                    let body = children
                        .iter()
                        .find_map(|child| match child {
                            Node::Tag {
                                name: "xml_body",
                                children,
                                ..
                            } => Some(children),
                            _ => None,
                        })
                        .unwrap();
                    Some(format!("{}{}", prefix, render_to_string(body)))
                }
                _ => None,
            };

            let mut children: Vec<TagTree<'_>> = children
                .iter()
                .flat_map(|child| lower_node(child).into_iter())
                .collect();

            if let Some(s) = tooltip {
                children = vec![TagTree::Tag {
                    tag: Tag::Tooltip(s.to_string()),
                    children,
                }];
            }

            if let Some(s) = class {
                children = vec![TagTree::Tag {
                    tag: Tag::SpanClass(s),
                    children,
                }];
            }

            children
        }
    }
}

fn main() -> io::Result<()> {
    let options: Options = argh::from_env();
    let yxml = std::fs::read_to_string(&options.dump_path)?;
    let nodes = yxml::parse(&yxml).unwrap();
    let ir = processed_ir(&nodes);
    let lines = split_lines(&ir);

    let mut writer = BufWriter::new(File::create(&options.out_path)?);

    write!(writer, "<!DOCTYPE html>")?;
    write!(writer, "<html>")?;
    write!(writer, "<head>")?;
    write!(writer, r#"<meta charset="utf-8">"#)?;
    write!(
        writer,
        r#"<link rel="stylesheet" type="text/css" href="../assets/isabelle.css">"#
    )?;
    write!(writer, "</head>")?;
    write!(writer, "<body>")?;
    write!(writer, r#"<pre class="isabelle-code">"#)?;

    for line in lines {
        write!(writer, "<code>")?;
        write_nodes(&mut writer, &line, false)?;
        write!(writer, "</code>")?;
    }
    write!(writer, "</pre></body></html>")?;
    Ok(())
}
