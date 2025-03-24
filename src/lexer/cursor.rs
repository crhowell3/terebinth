use std::str::Chars;

/// The `Cursor` is a peekable iterator for a character sequence.
///
/// The next character in a sequence can be "peeked" with `first`, and the
/// iterator can be progressed with `bump`.
pub struct Cursor<'a> {
    /// Remaining length of character sequence that has yet to be iterated over.
    remaining_len: usize,
    /// Characters are slightly faster to iterate over than &str.
    chars: Chars<'a>,
    #[cfg(debug_assertions)]
    prev: char,
}

pub(super) const EOF_CHAR: char = '\0';

impl<'a> Cursor<'a> {
    pub fn new(input: &'a str) -> Cursor<'a> {
        Cursor {
            remaining_len: input.len(),
            chars: input.chars(),
            prev: EOF_CHAR,
        }
    }

    /// Converts the characters to str and returns a reference with a lifetime.
    pub fn as_str(&self) -> &'a str {
        self.chars.as_str()
    }

    /// Last consumed symbol; really only used for debugging purposes.
    pub(super) fn prev(&self) -> char {
        #[cfg(debug_assertions)]
        {
            self.prev
        }

        #[cfg(not(debug_assertions))]
        {
            EOF_CHAR
        }
    }

    /// This method will peek the next symbol in the character sequence without
    /// that symbol being consumed by the lexer. If there is no "next" symbol,
    /// then an end of file is returned. This is just for safety; the EOF char
    /// does not necessarily mean that the iterator is at the end of the file.
    pub fn first(&self) -> char {
        self.chars.clone().next().unwrap_or(EOF_CHAR)
    }

    /// Peeks ahead two symbols, again without consuming the symbol.
    pub(super) fn second(&self) -> char {
        let mut iter = self.chars.clone();
        iter.next();
        iter.next().unwrap_or(EOF_CHAR)
    }

    /// Peeks ahead three symbols, again without consuming the symbol.
    pub fn third(&self) -> char {
        let mut iter = self.chars.clone();
        iter.next();
        iter.next();
        iter.next().unwrap_or(EOF_CHAR)
    }

    /// Checks if the iterator is at the end of the file and returns true if it
    /// is, false if it is not.
    pub(super) fn is_eof(&self) -> bool {
        self.chars.as_str().is_empty()
    }

    /// Returns the iterator's position, also implicitly being the number of
    /// consumed symbols.
    pub(super) fn pos_within_token(&self) -> u32 {
        (self.remaining_len - self.chars.as_str().len()) as u32
    }

    /// Resets the number of consumed bytes back to 0.
    pub(super) fn reset_pos_within_token(&mut self) {
        self.remaining_len = self.chars.as_str().len();
    }

    /// Moves the iterator to the next character.
    pub(super) fn bump(&mut self) -> Option<char> {
        let c = self.chars.next()?;

        #[cfg(debug_assertions)]
        {
            self.prev = c;
        }

        Some(c)
    }

    pub(super) fn consume_while(&mut self, mut predicate: impl FnMut(char) -> bool) {
        while predicate(self.first()) && !self.is_eof() {
            self.bump();
        }
    }

    pub(super) fn consume_until(&mut self, byte: u8) {
        self.chars = match memchr::memchr(byte, self.as_str().as_bytes()) {
            Some(index) => self.as_str()[index..].chars(),
            None => "".chars(),
        }
    }
}
