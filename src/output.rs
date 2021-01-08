//! This module takes care of the actual HTML output. The key non-trivial
//! behavior is the handling of newlines --- each line is in a separate `<code>` tag,
//! and when a newline occurs, all syntax highlighting tags must be closed, and then
//! reopened on the following line, to ensure proper tag nesting.

use std::fs::File;
use std::io::{self, BufWriter, prelude::*};
use std::path::Path;

pub struct HTMLOutput {
    writer: BufWriter<File>,
    tag_stack: Vec<Tag>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Tag {
    SpanClass(String),
}

impl Tag {
    fn open(&self, mut writer: impl Write) -> io::Result<()> {
        match self {
            Tag::SpanClass(class) => write!(writer, "<span class=\"{}\">", class),
        }
    }

    fn close(&self, mut writer: impl Write) -> io::Result<()> {
        match self {
            Tag::SpanClass(_) => write!(writer, "</span>"),
        }
    }
}

impl HTMLOutput {
    pub fn to_file(path: &Path) -> io::Result<Self> {
        let mut writer = HTMLOutput {
            writer: BufWriter::new(File::create(path)?),
            tag_stack: vec![],
        };

        writer.write_preamble()?;
        Ok(writer)
    }

    pub fn open_tag(&mut self, tag: Tag) -> io::Result<()> {
        tag.open(&mut self.writer)?;
        self.tag_stack.push(tag);
        Ok(())
    }

    pub fn close_tag(&mut self) -> io::Result<()> {
        self.tag_stack.pop().expect("tag stack underflow").close(&mut self.writer)
    }

    fn write_text_oneline(&mut self, s: &str) -> io::Result<()> {
        crate::symbols::render_symbols(s, &mut self.writer)
    }

    pub fn write_text(&mut self, s: &str) -> io::Result<()> {
        let mut lines = s.split("\n");
        self.write_text_oneline(lines.next().unwrap())?;
        for line in lines {
            self.handle_newline()?;
            self.write_text_oneline(line)?;
        }

        Ok(())
    }

    fn handle_newline(&mut self) -> io::Result<()> {
        for tag in self.tag_stack.iter().rev() {
            tag.close(&mut self.writer)?;
        }

        write!(self.writer, "</code>\n<code>")?;

        for tag in self.tag_stack.iter() {
            tag.open(&mut self.writer)?;
        }

        Ok(())
    }

    fn write_preamble(&mut self) -> io::Result<()> {
        write!(self.writer, "<!DOCTYPE html>")?;
        write!(self.writer, "<html>")?;
        write!(self.writer, "<head>")?;
        write!(self.writer, r#"<meta charset="utf-8">"#)?;
        write!(self.writer,
           r#"<link rel="stylesheet" type="text/css" href="../assets/isabelle.css">"#)?;
        write!(self.writer, "</head>")?;
        write!(self.writer, "<body>")?;
        write!(self.writer, r#"<pre class="isabelle-code">"#)?;
        write!(self.writer, "<code>")?;
        Ok(())
    }
}

impl Drop for HTMLOutput {
    fn drop(&mut self) {
        write!(self.writer, "</code></pre></body></html>").unwrap();
    }
}
