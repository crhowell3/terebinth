use crate::terebinth_span::{BytePos, Symbol};

use crate::token::CommentKind;

#[cfg(test)]
mod tests;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum CommentStyle {
    Isolated,
    Trailing,
    Mixed,
    BlankLine,
}

#[derive(Clone)]
pub struct Comment {
    pub style: CommentStyle,
    pub lines: Vec<String>,
    pub pos: BytePos,
}
