//! The tree-based intermediate representation.
//!
//! # Motivation
//!
//! Consider the following ML fragment:
//!
//! ```text
//! ML ‹f x›
//! ```
//!
//! Isabelle will generate the following `<xml_elem>` tags:
//! - on the `f`: type `a -> b`
//! - on the `x`: type `a`
//! - on the entire `f x`: type `b`
//!
//! There isn't really a good way of displaying all of these, so, like Isabelle/jEdit,
//! we only display the innermost tooltips, and ignore the `f x` one.
//!
//! To do this, we need a representation where all the different markup that may produce
//! a tooltip.

use crate::symbols::render_symbols;
use std::io;
use vec_mut_scan::VecGrowScan;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Tag {
    SpanClass(String),
    // contains processed HTML
    Tooltip(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TagTree<'a> {
    Tag {
        tag: Tag,
        children: Vec<TagTree<'a>>,
    },
    Text(&'a str),
}

impl<'a> TagTree<'a> {
    fn is_empty(&self) -> bool {
        match self {
            TagTree::Tag { children, .. } => children.is_empty(),
            TagTree::Text(s) => s.is_empty(),
        }
    }

    fn split_lines(&self) -> Vec<TagTree<'a>> {
        match self {
            TagTree::Text(s) => s.split('\n').map(TagTree::Text).collect(),
            TagTree::Tag { tag, children } => split_lines(&children)
                .into_iter()
                .map(|line| TagTree::Tag {
                    tag: tag.clone(),
                    children: line,
                })
                .collect(),
        }
    }
}

pub fn trim_empty(tree: &mut Vec<TagTree<'_>>) {
    tree.iter_mut().for_each(|node| {
        if let TagTree::Tag { children, .. } = node {
            trim_empty(children);
        }
    });
    tree.retain(|node| !node.is_empty());
}

/// Merge all tooltips on the same character range into one. Remove tooltips whose
/// ranges contain other tooltips.
///
/// Returns true if subtree contains tooltips after merging.
pub fn merge_tooltips<'a>(
    tree: &mut Vec<TagTree<'a>>,
    mut parent_tooltip: Option<&mut String>,
) -> bool {
    if let Some(parent_tooltip) = parent_tooltip {
        // The parent tooltip is only relevant when this is the only child
        if tree.len() == 1 {
            match &mut tree[0] {
                TagTree::Tag { tag, ref mut children } => {
                    match tag {
                        Tag::SpanClass(_) => {
                            return merge_tooltips(children, Some(parent_tooltip));
                        }
                        Tag::Tooltip(s) => {
                            parent_tooltip.push('\n');
                            parent_tooltip.push_str(&s);
                            // Obtain ownership of the children
                            if let TagTree::Tag { children, ..  } = tree.pop().unwrap() {
                                *tree = children;
                                return merge_tooltips(tree, Some(parent_tooltip));
                            } else {
                                unreachable!()
                            }
                        }
                    }
                }
                TagTree::Text(_) => return false,
            }
        }
    }

    let mut scan = VecGrowScan::new(tree);
    let mut any_tooltips = false;
    while let Some(mut node) = scan.next() {
        if let TagTree::Tag { tag, children } = &mut *node {
            let tooltip = if let Tag::Tooltip(ref mut s) = tag {
                Some(s)
            } else {
                None
            };
            let has_tooltips = merge_tooltips(children, tooltip);
            if let Tag::Tooltip(_) = tag {
                if has_tooltips {
                    node.replace_with_many_with(|node| {
                        if let TagTree::Tag { children, .. } = node {
                            children
                        } else {
                            unreachable!()
                        }
                    });
                } else {
                    any_tooltips = true;
                }
            }

            any_tooltips |= has_tooltips;
        }
    }

    any_tooltips
}

pub fn split_lines<'a>(input: &[TagTree<'a>]) -> Vec<Vec<TagTree<'a>>> {
    let mut lines = vec![];
    let mut new_children = vec![];
    for child in input {
        let child_lines = child.split_lines();
        let last_i = child_lines.len() - 1;
        for (i, child_line) in child_lines.into_iter().enumerate() {
            new_children.push(child_line);
            if i != last_i {
                lines.push(new_children);
                new_children = vec![];
            }
        }
    }

    lines.push(new_children);
    lines
}

pub fn write_nodes(
    writer: &mut impl io::Write,
    input: &[TagTree<'_>],
    in_tooltip: bool,
) -> io::Result<()> {
    for node in input {
        match node {
            TagTree::Text(s) => render_symbols(s, &mut *writer, !in_tooltip)?,
            TagTree::Tag { tag, children } => match tag {
                Tag::Tooltip(s) => {
                    assert!(!in_tooltip);
                    write!(writer, "<span class=\"has-tooltip\">")?;
                    write_nodes(writer, children, true)?;
                    write!(writer, "<span class=\"tooltip\">{}</span></span>", s)?;
                }
                Tag::SpanClass(cls) => {
                    write!(writer, "<span class=\"{}\">", cls)?;
                    write_nodes(writer, children, in_tooltip)?;
                    write!(writer, "</span>")?;
                }
            },
        }
    }

    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn split_lines() {
        let input = TagTree::Tag {
            tag: Tag::SpanClass("outer".to_owned()),
            children: vec![
                TagTree::Text("hi!"),
                TagTree::Text("one\ntwo"),
                TagTree::Tag {
                    tag: Tag::SpanClass("inner".to_owned()),
                    children: vec![TagTree::Text("and a half\nthree")],
                },
            ],
        };

        let output = vec![
            TagTree::Tag {
                tag: Tag::SpanClass("outer".to_owned()),
                children: vec![TagTree::Text("hi!"), TagTree::Text("one")],
            },
            TagTree::Tag {
                tag: Tag::SpanClass("outer".to_owned()),
                children: vec![
                    TagTree::Text("two"),
                    TagTree::Tag {
                        tag: Tag::SpanClass("inner".to_owned()),
                        children: vec![TagTree::Text("and a half")],
                    },
                ],
            },
            TagTree::Tag {
                tag: Tag::SpanClass("outer".to_owned()),
                children: vec![TagTree::Tag {
                    tag: Tag::SpanClass("inner".to_owned()),
                    children: vec![TagTree::Text("three")],
                }],
            },
        ];

        assert_eq!(input.split_lines(), output);
    }

    #[test]
    fn merge_tooltips_merges() {
        let mut input = vec![TagTree::Tag {
            tag: Tag::Tooltip("outer tooltip".to_owned()),
            children: vec![TagTree::Tag {
                tag: Tag::Tooltip("inner tooltip".to_owned()),
                children: vec![TagTree::Text("hi")],
            }],
        }];

        assert_eq!(merge_tooltips(&mut input, None), true);
        assert_eq!(
            input,
            [TagTree::Tag {
                tag: Tag::Tooltip("outer tooltip\ninner tooltip".to_owned()),
                children: vec![TagTree::Text("hi")],
            }]
        );
    }

    #[test]
    fn merge_tooltips_trims() {
        let mut input = vec![TagTree::Tag {
            tag: Tag::Tooltip("outer tooltip".to_owned()),
            children: vec![
                TagTree::Tag {
                    tag: Tag::Tooltip("inner tooltip".to_owned()),
                    children: vec![TagTree::Text("hi")],
                },
                TagTree::Text("some more text"),
            ],
        }];

        assert_eq!(merge_tooltips(&mut input, None), true);
        assert_eq!(
            input,
            [
                TagTree::Tag {
                    tag: Tag::Tooltip("inner tooltip".to_owned()),
                    children: vec![TagTree::Text("hi")],
                },
                TagTree::Text("some more text")
            ],
        );
    }

    #[test]
    fn merge_tooltips_merges_across_layers() {
        let mut input = vec![TagTree::Tag {
            tag: Tag::Tooltip("outer tooltip".to_owned()),
            children: vec![TagTree::Tag {
                tag: Tag::SpanClass("cls".to_owned()),
                children: vec![TagTree::Tag {
                    tag: Tag::Tooltip("inner tooltip".to_owned()),
                    children: vec![TagTree::Text("hi")],
                }],
            }],
        }];

        assert_eq!(merge_tooltips(&mut input, None), true);
        assert_eq!(
            input,
            [TagTree::Tag {
                tag: Tag::Tooltip("outer tooltip\ninner tooltip".to_owned()),
                children: vec![TagTree::Tag {
                    tag: Tag::SpanClass("cls".to_owned()),
                    children: vec![TagTree::Text("hi")],
                }],
            }]
        );
    }
}
