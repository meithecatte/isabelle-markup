use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Node<'a> {
    Text(&'a str),
    Tag {
        name: &'a str,
        attrs: HashMap<&'a str, &'a str>,
        children: Vec<Node<'a>>,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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

pub fn parse<'input>(mut input: &'input str)
    -> Result<Vec<Node<'input>>, ParseError<'input>>
{
    let mut nodes = Vec::new();
    while !input.is_empty() {
        let (node, rest) = Node::from_str(input)?;
        input = rest;
        nodes.push(node.ok_or(ParseError::UnmatchedClosingTag)?);
    }

    Ok(nodes)
}

fn parse_children<'input>(tag: &'input str, mut input: &'input str)
    -> ParseResult<'input, Vec<Node<'input>>>
{
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
    fn from_str<'input>(input: &'input str)
        -> ParseResult<'input, Option<Node<'input>>>
    {
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
                    let attrs = attributes.map(|attr| {
                        let offset = attr.find('=').ok_or(ParseError::MalformedAttribute)?;
                        Ok((&attr[0..offset], &attr[offset+1..]))
                    }).collect::<Result<_, _>>()?;

                    let (children, rest) = parse_children(name, rest)?;
                    Ok((Some(Node::Tag { name, attrs, children }), rest))
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
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
