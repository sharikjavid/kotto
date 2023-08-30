use std::fmt::{Display, Formatter};
use std::ops::Deref;
use serde::{Serialize, Deserialize};

use crate::{codegen, emit, ast};
use crate::common::comments::Comments;
use crate::CanPush;

#[derive(Debug)]
pub enum InvalidPromptError {
    InvalidId(String),
}

impl Display for InvalidPromptError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidId(id) => write!(f, "invalid id: {}", id)
        }
    }
}

impl std::error::Error for InvalidPromptError {}

#[derive(Serialize, Deserialize)]
#[serde(transparent)]
pub struct Prompts(pub Vec<Prompt>);

#[derive(Serialize, Deserialize)]
#[serde(transparent)]
pub struct PromptFmt(pub String);

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PromptAstType {
    MethodDecl,
    ClassDecl,
    TypeAliasDecl,
    FnDecl,
}

#[derive(Serialize, Deserialize)]
#[serde(transparent)]
pub struct PromptId(pub String);

impl Deref for PromptId {
    type Target = str;

    fn deref(&self) -> &Self::Target { &self.0 }
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PromptType {
    PlainText,
    #[serde(rename = "ts")]
    TypeScript
}

#[derive(Serialize, Deserialize)]
pub struct Prompt {
    #[serde(rename = "type")]
    pub ty: PromptType,
    pub fmt: PromptFmt,
    pub id: PromptId,
    pub ast_ty: Option<PromptAstType>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub context: Vec<PromptId>,
}

impl Default for Prompt {
    fn default() -> Self {
        Self {
            ty: PromptType::PlainText,
            fmt: PromptFmt(String::new()),
            ast_ty: None,
            id: PromptId(String::new()),
            context: Vec::new()
        }
    }
}

pub struct PromptsWriter<'p, C> {
    buf: &'p mut C,
    modified: bool,
    comments: &'p dyn Comments,
    scope: Vec<String>,
    builder: Prompt
}

impl<'p, C> PromptsWriter<'p, C> {
    pub fn new(buf: &'p mut C, comments: &'p dyn Comments) -> Self {
        Self {
            buf,
            modified: false,
            comments,
            scope: Vec::default(),
            builder: Prompt::default()
        }
    }

    pub fn set_type(&mut self, prompt_type: PromptType) {
        self.modified = true;
        self.builder.ty = prompt_type;
    }

    pub fn set_fmt<N: codegen::Node>(&mut self, node: &N) -> Result<(), InvalidPromptError> {
        let mut buf = Vec::new();
        let mut emitter = emit::Emitter::new(&mut buf)
            .with_comments(&self.comments);
        node.emit_with(&mut emitter).unwrap();
        let source_text = String::from_utf8(buf).unwrap();

        self.modified = true;
        self.builder.fmt = PromptFmt(source_text);

        Ok(())
    }

    pub fn add_to_context<I, S>(&mut self, iter: I) -> Result<(), InvalidPromptError>
    where
        I: Iterator<Item = S>,
        S: AsRef<str>
    {
        self.builder.context.extend(iter.map(|s| PromptId(s.as_ref().into())));
        Ok(())
    }

    pub fn enter_scope(&mut self, scope: &ast::Ident) -> () {
        self.scope.push(format!("{}", scope));
    }

    pub fn exit_scope(&mut self) -> Option<String> {
        self.scope.pop()
    }

    pub fn set_id(&mut self, id: &ast::Ident) {
        self.modified = true;
        self.builder.id = PromptId(format!("{}", id));
    }

    pub fn set_ast_ty(&mut self, ast_ty: PromptAstType) {
        self.modified = true;
        self.builder.ast_ty = Some(ast_ty);
    }
}

impl<'p, C> PromptsWriter<'p, C>
where
   C: CanPush<Prompt>
{
    pub fn push(&mut self) -> Result<(), InvalidPromptError> {
        if !self.modified {
            return Ok(())
        }

        if self.builder.id.is_empty() {
            return Err(InvalidPromptError::InvalidId(self.builder.id.to_string()))
        }

        let mut prompt = std::mem::replace(&mut self.builder, Prompt::default());

        if !self.scope.is_empty() {
            prompt.id = PromptId(format!("{}.{}", self.scope.join("."), &*prompt.id));
        }

        self.buf.push(prompt);

        self.modified = false;

        Ok(())
    }
}