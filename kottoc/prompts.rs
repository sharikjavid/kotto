use std::fmt::{Display, Formatter};
use std::ops::Deref;
use serde::{Serialize, Deserialize};

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
#[serde(transparent)]
pub struct PromptId(pub String);

impl Deref for PromptId {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
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
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub context: Vec<PromptId>,
}

impl Default for Prompt {
    fn default() -> Self {
        Self {
            ty: PromptType::PlainText,
            fmt: PromptFmt(String::new()),
            id: PromptId(String::new()),
            context: Vec::new()
        }
    }
}

pub struct PromptsWriter<'p, C> {
    buf: &'p mut C,
    modified: bool,
    builder: Prompt
}

impl<'p, C> PromptsWriter<'p, C> {
    pub fn new(buf: &'p mut C) -> Self {
        Self {
            buf,
            modified: false,
            builder: Prompt::default()
        }
    }

    pub fn set_type(&mut self, prompt_type: PromptType) {
        self.modified = true;
        self.builder.ty = prompt_type;
    }

    pub fn set_fmt<S: AsRef<str>>(&mut self, fmt: S) -> Result<(), InvalidPromptError> {
        self.modified = true;
        self.builder.fmt = PromptFmt(fmt.as_ref().into());
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

    pub fn set_id<S: AsRef<str>>(&mut self, id: S) -> Result<(), InvalidPromptError> {
        self.modified = true;
        self.builder.id = PromptId(id.as_ref().into());
        Ok(())
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

        let prompt = std::mem::replace(&mut self.builder, Prompt::default());

        self.buf.push(prompt);

        self.modified = false;

        Ok(())
    }
}