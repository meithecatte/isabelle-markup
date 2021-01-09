//! This module takes care of the actual HTML output. The key non-trivial
//! behavior is the handling of newlines --- each line is in a separate `<code>` tag,
//! and when a newline occurs, all syntax highlighting tags must be closed, and then
//! reopened on the following line, to ensure proper tag nesting.

use std::fs::File;
use std::io::{self, prelude::*, BufWriter};
use std::path::Path;

enum TooltipState {
    None,
    Pending(String),
    Emitted,
}

pub struct HTMLOutput<'a> {
    writer: Box<dyn Write + 'a>,
    tag_stack: Vec<Tag>,
    tooltip: TooltipState,
    /// If `true`, this is the topmost instance of `HTMLOutput` (as opposed to the contents
    /// of a tooltip). This variable determines whether lines should be split across `<code>`
    /// tags, as well as whether the epilogue should be written out on drop.
    root: bool,
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

impl<'a> HTMLOutput<'a> {
    pub fn to_file(path: &Path) -> io::Result<HTMLOutput<'static>> {
        let mut writer = HTMLOutput {
            writer: Box::new(BufWriter::new(File::create(path)?)),
            tag_stack: vec![],
            tooltip: TooltipState::None,
            root: true,
        };

        writer.write_preamble()?;
        Ok(writer)
    }

    pub fn into_buffer<'buf>(buf: &'buf mut Vec<u8>) -> HTMLOutput<'buf> {
        HTMLOutput {
            writer: Box::new(io::Cursor::new(buf)),
            tag_stack: vec![],
            tooltip: TooltipState::None,
            root: false,
        }
    }

    pub fn open_tag(&mut self, tag: Tag) -> io::Result<()> {
        tag.open(&mut self.writer)?;
        self.tag_stack.push(tag);
        Ok(())
    }

    pub fn close_tag(&mut self) -> io::Result<()> {
        self.tag_stack
            .pop()
            .expect("tag stack underflow")
            .close(&mut self.writer)
    }

    fn write_text_oneline(&mut self, s: &str) -> io::Result<()> {
        crate::symbols::render_symbols(s, &mut self.writer, true)
    }

    pub fn write_text(&mut self, s: &str) -> io::Result<()> {
        let mut lines = s.split('\n');
        self.write_text_oneline(lines.next().unwrap())?;
        for line in lines {
            self.handle_newline()?;
            self.write_text_oneline(line)?;
        }

        Ok(())
    }

    fn handle_newline(&mut self) -> io::Result<()> {
        if self.root {
            for tag in self.tag_stack.iter().rev() {
                tag.close(&mut self.writer)?;
            }

            write!(self.writer, "</code>\n<code>")?;

            for tag in self.tag_stack.iter() {
                tag.open(&mut self.writer)?;
            }
        } else {
            write!(self.writer, "\n")?;
        }

        Ok(())
    }

    fn write_preamble(&mut self) -> io::Result<()> {
        write!(self.writer, "<!DOCTYPE html>")?;
        write!(self.writer, "<html>")?;
        write!(self.writer, "<head>")?;
        write!(self.writer, r#"<meta charset="utf-8">"#)?;
        write!(
            self.writer,
            r#"<link rel="stylesheet" type="text/css" href="../assets/isabelle.css">"#
        )?;
        write!(self.writer, "</head>")?;
        write!(self.writer, "<body>")?;
        write!(self.writer, r#"<pre class="isabelle-code">"#)?;
        write!(self.writer, "<code>")?;
        Ok(())
    }
}

impl<'a> Drop for HTMLOutput<'a> {
    fn drop(&mut self) {
        if self.root {
            write!(self.writer, "</code></pre></body></html>").unwrap();
        }
    }
}
