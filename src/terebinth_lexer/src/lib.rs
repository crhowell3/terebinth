//! The low-level Terebinth lexer.
//!
//! The lexer tokenizes the Terebinth source code so that the parser can
//! construct the abstract syntax tree. Each token is categorized by type,
//! and invalid tokens are marked as such.

mod cursor;

pub use literal_escaper as unescape;
use unicode_properties::UnicodeEmoji;
pub use unicode_xid::UNICODE_VERSION as UNICODE_XID_VERSION;

use self::LiteralKind::*;
use self::TokenKind::*;
pub use crate::cursor::Cursor;
use crate::cursor::EOF_CHAR;

/// A Token in Terebinth is an identifier, keyword, operator, or symbol.
/// The Token will contain token type information as well as its length.
#[derive(Debug, PartialEq, Clone)]
pub struct Token {
    pub kind: TokenKind,
    pub len: u32,
}

impl Token {
    pub fn new(kind: TokenKind, len: u32) -> Self {
        Self { kind, len }
    }
}

/// Types of lexeme.
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum TokenKind {
    /// Standard line comment, e.g., `// a comment`.
    LineComment {
        doc_style: Option<DocStyle>,
    },

    /// Standard block comment, e.g., `/* block comment*/`.
    ///
    /// As is the case with some languages, Terebinth's block comments will be
    /// recursive (meaning they can be nested).
    BlockComment {
        doc_style: Option<DocStyle>,
        terminated: bool,
    },

    /// Any whitespace character sequence.
    Whitespace,

    /// A keyword or an identifier, i.e., a variable name.
    Ident,

    /// An identifier that contains an emoji, rendering it invalid.
    InvalidIdent,

    /// An unknown literal prefix.
    UnknownPrefix,

    /// Literals, namely numeric. Underscores are invalid suffixes, but they may
    /// be present on string and float literals.
    ///
    /// See [LiteralKind] for more details.
    Literal {
        kind: LiteralKind,
        suffix_start: u32,
    },
    /// `+`
    Plus,
    /// `-`
    Minus,
    /// `*`
    Star,
    /// `/`
    Slash,
    /// `=`
    Eq,
    /// `&`
    Ampersand,
    /// `|`
    Pipe,
    /// `^`
    Caret,
    /// `**`
    DoubleAsterisk,
    /// `~`
    Tilde,
    /// `<<`
    DoubleLessThan,
    /// `>>`
    DoubleGreaterThan,
    /// `==`
    EqualsEquals,
    /// `!=`
    BangEquals,
    /// `<`
    Lt,
    /// `<=`
    LessThanEquals,
    /// `>`
    Gt,
    /// `>=`
    GreaterThanEquals,
    /// `(`
    OpenParen,
    /// `)`
    CloseParen,
    /// `{`
    OpenBrace,
    /// `}`
    CloseBrace,
    /// `[`
    OpenBracket,
    /// ']'
    CloseBracket,
    /// `,`
    Comma,
    /// `:`
    Colon,
    /// `->`
    Arrow,
    /// `;`
    Semi,
    /// `.`
    Dot,
    /// `@`
    At,
    /// `#`
    Octothorpe,
    /// `?`
    Eroteme,
    /// `!`
    Bang,
    /// `$`
    Dollar,
    /// `%`
    Percent,

    /// An unknown token not recognized by the compiler
    Invalid,
    Eof,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DocStyle {
    Outer,
    Inner,
}

/// This enum spells out the literal types that are supported by the lexer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum LiteralKind {
    Int { base: Base, empty_int: bool },
    Float { base: Base, empty_exponent: bool },
    Char { terminated: bool },
    Byte { terminated: bool },
    Str { terminated: bool },
}

/// This describes the numeric base of a numbe based on the literal's prefix.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Base {
    /// Literal starting with "0b".
    Binary = 2,
    /// Literal starting with "0o".
    Octal = 8,
    /// Literal without a prefix (meaning decimal is the default for numeric literals).
    Decimal = 10,
    /// Literal starting with "0x".
    Hexadecimal = 16,
}

/// This creates an iterator that produces tokens from the input string.
pub fn tokenize(input: &str) -> impl Iterator<Item = Token> {
    let mut cursor = Cursor::new(input);
    std::iter::from_fn(move || {
        let token = cursor.advance_token();
        if token.kind != TokenKind::Eof {
            Some(token)
        } else {
            None
        }
    })
}

pub fn is_whitespace(c: char) -> bool {
    c.is_whitespace()
}

pub fn is_identifier_start(c: char) -> bool {
    c.is_alphabetic()
}

pub fn is_identifier_continue(c: char) -> bool {
    unicode_xid::UnicodeXID::is_xid_continue(c)
}

/// This returns true if the string is determined to be an identifier
pub fn is_identifier(string: &str) -> bool {
    let mut chars = string.chars();
    if let Some(start) = chars.next() {
        is_identifier_start(start) && chars.all(is_identifier_continue)
    } else {
        false
    }
}

impl Cursor<'_> {
    pub fn advance_token(&mut self) -> Token {
        let first_char = match self.bump() {
            Some(c) => c,
            None => return Token::new(TokenKind::Eof, 0),
        };
        let token_kind = match first_char {
            '/' => match self.first() {
                '/' => self.line_comment(),
                '*' => self.block_comment(),
                _ => Slash,
            },

            c if is_whitespace(c) => self.whitespace(),

            c if is_identifier_start(c) => self.identifier_or_unknown_prefix(),

            c @ '0'..='9' => {
                let literal_kind = self.number(c);
                let suffix_start = self.pos_within_token();
                self.consume_literal_suffix();
                TokenKind::Literal {
                    kind: literal_kind,
                    suffix_start,
                }
            }

            ';' => Semi,
            ',' => Colon,
            '.' => Dot,
            '(' => OpenParen,
            ')' => CloseParen,
            '{' => OpenBrace,
            '}' => CloseBrace,
            '[' => OpenBracket,
            ']' => CloseBracket,
            '@' => At,
            '#' => Octothorpe,
            '~' => Tilde,
            '?' => Eroteme,
            ':' => Colon,
            '$' => Dollar,
            '=' => Eq,
            '!' => Bang,
            '<' => Lt,
            '>' => Gt,
            '-' => Minus,
            '&' => Ampersand,
            '|' => Pipe,
            '+' => Plus,
            '*' => Star,
            '^' => Caret,
            '%' => Percent,

            c if !c.is_ascii() => self.invalid_identifier(),
            _ => Invalid,
        };

        let res = Token::new(token_kind, self.pos_within_token());
        self.reset_pos_within_token();
        res
    }

    fn line_comment(&mut self) -> TokenKind {
        debug_assert!(self.prev() == '/' && self.first() == '/');
        self.bump();

        let doc_style = match self.first() {
            '!' => Some(DocStyle::Inner),
            '/' if self.second() != '/' => Some(DocStyle::Outer),
            _ => None,
        };

        self.consume_until(b'\n');
        LineComment { doc_style }
    }

    fn block_comment(&mut self) -> TokenKind {
        debug_assert!(self.prev() == '/' && self.first() == '*');
        self.bump();

        let doc_style = match self.first() {
            '!' => Some(DocStyle::Inner),
            '*' if !matches!(self.second(), '*' | '/') => Some(DocStyle::Outer),
            _ => None,
        };

        let mut depth = 1usize;
        while let Some(c) = self.bump() {
            match c {
                '/' if self.first() == '*' => {
                    self.bump();
                    depth += 1;
                }
                '*' if self.first() == '/' => {
                    self.bump();
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                }
                _ => (),
            }
        }

        BlockComment {
            doc_style,
            terminated: depth == 0,
        }
    }

    fn whitespace(&mut self) -> TokenKind {
        debug_assert!(is_whitespace(self.prev()));
        self.consume_while(is_whitespace);
        Whitespace
    }

    fn identifier_or_unknown_prefix(&mut self) -> TokenKind {
        debug_assert!(is_identifier_start(self.prev()));
        self.consume_while(is_identifier_continue);
        match self.first() {
            '#' | '"' | '\'' => UnknownPrefix,
            c if !c.is_ascii() => self.invalid_identifier(),
            _ => Ident,
        }
    }

    fn invalid_identifier(&mut self) -> TokenKind {
        self.consume_while(|c| {
            const ZERO_WIDTH_JOINER: char = '\u{200d}';
            is_identifier_continue(c) || (!c.is_ascii()) || c == ZERO_WIDTH_JOINER
        });

        InvalidIdent
    }

    fn number(&mut self, first_digit: char) -> LiteralKind {
        debug_assert!('0' <= self.prev() && self.prev() <= '9');
        let mut base = Base::Decimal;
        if first_digit == '0' {
            match self.first() {
                'b' => {
                    base = Base::Binary;
                    self.bump();
                    if !self.consume_decimal_digits() {
                        return Int {
                            base,
                            empty_int: true,
                        };
                    }
                }
                'o' => {
                    base = Base::Octal;
                    self.bump();
                    if !self.consume_decimal_digits() {
                        return Int {
                            base,
                            empty_int: true,
                        };
                    }
                }
                'x' => {
                    base = Base::Hexadecimal;
                    self.bump();
                    if !self.consume_hexadecimal_digits() {
                        return Int {
                            base,
                            empty_int: true,
                        };
                    }
                }
                '0'..='9' | '_' => {
                    self.consume_decimal_digits();
                }

                '.' | 'e' | 'E' => {}

                _ => {
                    return Int {
                        base,
                        empty_int: false,
                    };
                }
            }
        } else {
            self.consume_decimal_digits();
        };

        match self.first() {
            '.' if self.second() != '.' && !is_identifier_start(self.second()) => {
                self.bump();
                let mut empty_exponent = false;
                if self.first().is_ascii_digit() {
                    self.consume_decimal_digits();
                    match self.first() {
                        'e' | 'E' => {
                            self.bump();
                            empty_exponent = !self.consume_float_exponent();
                        }
                        _ => (),
                    }
                }
                Float {
                    base,
                    empty_exponent,
                }
            }
            'e' | 'E' => {
                self.bump();
                let empty_exponent = !self.consume_float_exponent();
                Float {
                    base,
                    empty_exponent,
                }
            }
            _ => Int {
                base,
                empty_int: false,
            },
        }
    }

    fn single_quoted_string(&mut self) -> bool {
        debug_assert!(self.prev() == '\'');
        if self.second() == '\'' && self.first() != '\\' {
            self.bump();
            self.bump();
            return true;
        }

        loop {
            match self.first() {
                '\'' => {
                    self.bump();
                    return true;
                }
                '/' => break,
                '\n' if self.second() != '\'' => break,
                EOF_CHAR if self.is_eof() => break,
                '\\' => {
                    self.bump();
                    self.bump();
                }
                _ => {
                    self.bump();
                }
            }
        }
        false
    }

    fn double_quoted_string(&mut self) -> bool {
        debug_assert!(self.prev() == '"');
        while let Some(c) = self.bump() {
            match c {
                '"' => {
                    return true;
                }
                '\\' if self.first() == '\\' || self.first() == '"' => {
                    self.bump();
                }
                _ => (),
            }
        }
        false
    }

    fn consume_decimal_digits(&mut self) -> bool {
        let mut has_digits = false;
        loop {
            match self.first() {
                '_' => {
                    self.bump();
                }
                '0'..='9' => {
                    has_digits = true;
                    self.bump();
                }
                _ => break,
            }
        }
        has_digits
    }

    fn consume_hexadecimal_digits(&mut self) -> bool {
        let mut has_digits = false;
        loop {
            match self.first() {
                '_' => {
                    self.bump();
                }
                '0'..='9' | 'a'..='f' | 'A'..='F' => {
                    has_digits = true;
                    self.bump();
                }
                _ => break,
            }
        }
        has_digits
    }

    fn consume_float_exponent(&mut self) -> bool {
        debug_assert!(self.prev() == 'e' || self.prev() == 'E');
        if self.first() == '-' || self.first() == '+' {
            self.bump();
        }
        self.consume_decimal_digits()
    }

    fn consume_literal_suffix(&mut self) {
        self.consume_identifier();
    }

    fn consume_identifier(&mut self) {
        if !is_identifier_start(self.first()) {
            return;
        }
        self.bump();
        self.consume_while(is_identifier_continue);
    }
}
