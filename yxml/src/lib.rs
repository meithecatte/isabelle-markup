use std::collections::HashMap;

/// A node of the parsed YXML tree
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Node<'a> {
    Text(&'a str),
    Tag {
        name: &'a str,
        attrs: HashMap<&'a str, &'a str>,
        children: Vec<Node<'a>>,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ParseError<'a> {
    UnclosedTag(&'a str),
    NoClosingX,
    UnexpectedContentBeforeAttributes,
    MissingName,
    MalformedAttribute,
    UnmatchedClosingTag,
}

const X: char = '\x05';
const Y: char = '\x06';

type ParseResult<'a, T> = Result<(T, &'a str), ParseError<'a>>;

pub fn parse<'input>(mut input: &'input str) -> Result<Vec<Node<'input>>, ParseError<'input>> {
    let mut nodes = Vec::new();
    while !input.is_empty() {
        let (node, rest) = Node::from_str(input)?;
        input = rest;
        nodes.push(node.ok_or(ParseError::UnmatchedClosingTag)?);
    }

    Ok(nodes)
}

fn parse_children<'input>(
    tag: &'input str,
    mut input: &'input str,
) -> ParseResult<'input, Vec<Node<'input>>> {
    let mut children = Vec::new();
    loop {
        if input.is_empty() {
            return Err(ParseError::UnclosedTag(tag));
        }

        let (child, rest) = Node::from_str(input)?;
        input = rest;
        if let Some(child) = child {
            children.push(child);
        } else {
            break;
        }
    }

    Ok((children, input))
}

impl<'a> Node<'a> {
    fn from_str<'input>(input: &'input str) -> ParseResult<'input, Option<Node<'input>>> {
        match input.find(X) {
            Some(0) => {
                let end = input[1..].find(X).ok_or(ParseError::NoClosingX)?;
                let (attributes, rest) = input[1..].split_at(end);
                let rest = &rest[1..];
                if attributes == "\x06" {
                    Ok((None, rest))
                } else {
                    let mut attributes = attributes.split(Y);
                    if attributes.next() != Some("") {
                        return Err(ParseError::UnexpectedContentBeforeAttributes);
                    }

                    let name = attributes.next().ok_or(ParseError::MissingName)?;
                    let attrs = attributes
                        .map(|attr| {
                            let offset = attr.find('=').ok_or(ParseError::MalformedAttribute)?;
                            Ok((&attr[0..offset], &attr[offset + 1..]))
                        })
                        .collect::<Result<_, _>>()?;

                    let (children, rest) = parse_children(name, rest)?;
                    Ok((
                        Some(Node::Tag {
                            name,
                            attrs,
                            children,
                        }),
                        rest,
                    ))
                }
            }
            Some(n) => {
                let (text, rest) = input.split_at(n);
                Ok((Some(Node::Text(text)), rest))
            }
            None => Ok((Some(Node::Text(input)), "")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // https://stackoverflow.com/a/27582993
    macro_rules! map(
        { $($key:expr => $value:expr),* } => {
            {
                #[allow(unused_mut)]
                let mut m = ::std::collections::HashMap::new();
                $(
                    m.insert($key, $value);
                )*
                m
            }
         };
    );

    #[test]
    fn it_works() {
        assert_eq!(
            parse("\x05\x06tag\x05hi\x05\x06\x05"),
            Ok(vec![Node::Tag {
                name: "tag",
                attrs: map!{},
                children: vec![Node::Text("hi")]
            }])
        );
    }

    #[test]
    fn equal_sign_in_attribute() {
        assert_eq!(
            parse("\x05\x06tag\x06attr=2+2=4\x05hi\x05\x06\x05"),
            Ok(vec![Node::Tag {
                name: "tag",
                attrs: map!{ "attr" => "2+2=4" },
                children: vec![Node::Text("hi")]
            }])
        );
    }

    #[test]
    fn unclosed_tag() {
        assert_eq!(
            parse("\x05\x06tag\x05hi"),
            Err(ParseError::UnclosedTag("tag"))
        );
    }

    #[test]
    fn no_closing_x() {
        assert_eq!(
            parse("\x05\x06tag"),
            Err(ParseError::NoClosingX)
        );
    }

    #[test]
    fn unexpected_content_before_attributes() {
        assert_eq!(
            parse("\x05xxx\x06tag\x05hi\x05\x06\x05"),
            Err(ParseError::UnexpectedContentBeforeAttributes)
        );
    }

    #[test]
    fn missing_name() {
        assert_eq!(
            parse("\x05\x05hi\x05\x06\x05"),
            Err(ParseError::MissingName)
        );
    }

    #[test]
    fn malformed_attribute() {
        assert_eq!(
            parse("\x05\x06tag\x06bad_attr\x05hi\x05\x06\x05"),
            Err(ParseError::MalformedAttribute)
        );
    }

    #[test]
    fn unmatched_closing_tag() {
        assert_eq!(
            parse("\x05\x06tag\x05hi\x05\x06\x05\x05\x06\x05"),
            Err(ParseError::UnmatchedClosingTag)
        );
    }
}
