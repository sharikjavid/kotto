use std::io::Write;
use std::ops::{Deref, DerefMut};
use std::rc::Rc;

use deno_ast::swc::common::SourceMap;

use crate::codegen;
use crate::common;

pub struct Emitter<'a, W: Write>(
    pub codegen::Emitter<'a, codegen::text_writer::JsWriter<'a, W>, SourceMap>
);

impl<'a, W: Write> Deref for Emitter<'a, W> {
    type Target = codegen::Emitter<'a, codegen::text_writer::JsWriter<'a, W>, SourceMap>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a, W: Write> DerefMut for Emitter<'a, W> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<'a, W: Write> Emitter<'a, W> {
    pub fn new(write: W) -> Self {
        // We don't need to refer back to the original file (yet) so just allocate
        // a default source map and hope for the best.
        let source_map = Rc::new(SourceMap::default());

        Self(
            codegen::Emitter {
                cfg: codegen::Config::default(),
                cm: source_map.clone(),
                comments: None,
                wr: codegen::text_writer::JsWriter::new(source_map, "\n", write, None)
            }
        )
    }

    pub fn with_comments(mut self, comments: &'a dyn common::comments::Comments) -> Self {
        self.comments = Some(comments);
        self
    }
}