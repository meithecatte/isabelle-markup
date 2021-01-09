use itertools::Itertools;
use once_cell::sync::Lazy;
use regex::Regex;
use std::collections::HashMap;
use std::convert::TryInto;
use std::io::{self, prelude::*};

#[derive(Debug)]
pub struct Symbol {
    unicode: Option<char>,
    name: &'static str,
    abbrev: Vec<&'static str>,
}

impl Symbol {
    fn tooltip(&self) -> String {
        let mut tooltip = format!("\\<{}>", self.name);
        for abbrev in &self.abbrev {
            tooltip.push_str("\nabbreviation: ");
            tooltip.push_str(abbrev);
        }
        html_escape::encode_text(&tooltip).into_owned()
    }

    fn write(&self, mut w: impl Write, with_tooltips: bool) -> io::Result<()> {
        if with_tooltips {
            let tooltip = format!(r#"<span class="tooltip">{}</span>"#, self.tooltip());
            if let Some(c) = self.unicode {
                write!(w, r#"<span class="has-tooltip">{}{}</span>"#, c, tooltip)
            } else {
                assert!(self.name.starts_with('^'));
                write!(
                    w,
                    r#"<span class="control has-tooltip">{}{}</span>"#,
                    &self.name[1..],
                    tooltip
                )
            }
        } else {
            if let Some(c) = self.unicode {
                write!(w, "{}", c)
            } else {
                assert!(self.name.starts_with('^'));
                write!(w, r#"<span class="control">{}</span>"#, &self.name[1..])
            }
        }
    }
}

static SYMBOL_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"\\<([a-zA-Z_^]+)>").unwrap());

static SYMBOLS: Lazy<HashMap<&'static str, Symbol>> = Lazy::new(parse_symbols);

fn parse_symbols() -> HashMap<&'static str, Symbol> {
    static SYMBOL_DATA: &str = include_str!("symbols");

    let mut symbols = HashMap::new();

    for line in SYMBOL_DATA.split('\n') {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let mut parts = line.split_whitespace();
        let symbol = parts.next().unwrap();
        let captures = SYMBOL_RE.captures(symbol).unwrap();
        assert_eq!(captures.get(0).unwrap().range(), 0..symbol.len());
        let name = captures.get(1).unwrap().as_str();

        let mut symbol = Symbol {
            name,
            unicode: None,
            abbrev: vec![],
        };

        for mut args in &parts.chunks(2) {
            let arg: &str = args.next().unwrap();
            let val: &str = args.next().unwrap();
            assert!(arg.ends_with(':'));
            match arg {
                "code:" => {
                    assert!(val.starts_with("0x"));
                    let val = &val[2..];
                    let num = u32::from_str_radix(val, 16).unwrap();
                    symbol.unicode = Some(num.try_into().unwrap());
                }
                "abbrev:" => symbol.abbrev.push(val),
                "group:" | "argument:" | "font:" => (),
                _ => panic!("Unknown argument: {:?}", arg),
            }
        }

        symbols
            .insert(name, symbol)
            .map(|_| panic!("Multiple symbols with the same name"));
    }

    symbols
}

pub fn render_symbols(s: &str, mut w: impl Write, with_tooltips: bool) -> io::Result<()> {
    let mut last_symbol = 0;
    for captures in SYMBOL_RE.captures_iter(s) {
        let range = captures.get(0).unwrap().range();
        let symbol = &SYMBOLS[&captures[1]];
        write!(
            w,
            "{}",
            html_escape::encode_text(&s[last_symbol..range.start]),
        )?;
        symbol.write(&mut w, with_tooltips)?;
        last_symbol = range.end;
    }
    write!(w, "{}", html_escape::encode_text(&s[last_symbol..]))
}
