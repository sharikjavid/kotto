use std::io::{self, Cursor, Write, BufRead};
use std::convert::identity;
use std::fmt::{Display, Formatter};
use std::rc::Rc;
use deno_ast::swc::common::{source_map::Pos, Span};

use colored::*;

pub enum Level {
    INFO,
    WARN,
    ERROR
}

impl Display for Level {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let level = match self {
            Self::INFO => "INFO".white(),
            Self::WARN => "WARN".yellow(),
            Self::ERROR => "ERROR".red()
        }.bold();
        write!(f, "{level}")
    }
}

pub struct LineSpan {
    pub lo: u64,
    pub row: String
}

pub struct CompileMessageEmitter<W> {
    filename: String,
    splits: Vec<LineSpan>,
    writer: W
}

/// TODO(brokad) handle unicode chars
impl<W> CompileMessageEmitter<W> {
    pub fn new<S: AsRef<str>>(filename: S, src: &str, writer: W) -> Result<Self, io::Error> {
        let mut reader = Cursor::new(src);
        let mut splits = Vec::new();
        let mut line_begin = 0;
        let mut buf = String::new();
        while reader.read_line(&mut buf)? != 0 {
            splits.push(LineSpan {
                lo: line_begin,
                row: buf.clone()
            });
            buf.clear();
            line_begin = reader.position();
        }

        Ok(Self {
            filename: filename.as_ref().to_string(),
            splits,
            writer
        })
    }

    pub fn locate(&self, at: u64) -> (usize, usize) {
        let mut idx = self.splits.binary_search_by_key(&at, |span| span.lo).unwrap_or_else(identity);
        let span = &self.splits[idx - 1];
        let offset = at - span.lo;
        (idx, offset as usize)
    }
}

impl<W> CompileMessageEmitter<W>
    where
        W: Write
{
    pub fn write<D: Display>(&mut self, span: &Span, level: Level, msg: D) -> Result<(), io::Error> {
        let (linenum, colnum) = self.locate(span.lo.to_u32() as u64);
        let prefix = format!(
            "{filename} [L{linenum}:{colnum}]",
            filename=&self.filename
        ).bold().dimmed();
        writeln!(self.writer, "{prefix} {level} {msg}")
    }
}

pub trait Emitter {
    fn emit(&mut self, span: &Span, level: Level, msg: &str);
}

impl<W> Emitter for CompileMessageEmitter<W>
    where
        W: Write
{
    fn emit(&mut self, span: &Span, level: Level, msg: &str) {
        let _ = self.write(span, level, msg);
    }
}