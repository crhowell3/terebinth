//! AST literal parsing

pub fn escape_string_symbol(symbol: Symbol) -> Symbol {
    let s = symbol.as_str();
    let escaped = s.escape_default().to_string();
    if s == escaped {
        symbol
    } else {
        Symbol::intern(&escaped)
    }
}

pub fn escape_char_symbol(ch: char) -> Symbol {
    let s: String = ch.escape_default().map(Into::<char>::into).collect();
    Symbol::intern(&s)
}

pub fn escape_byte_str_symbol(bytes: &[u8]) -> Symbol {
    let s = bytes.escape_ascii().to_string();
    Symbol::intern(&s)
}

#[derive(Debug)]
pub enum LitError {
    InvalidSuffix(Symbol),
    InvalidIntSuffix(Symbol),
    InvalidFloatSuffix(Symbol),
    NonDecimalFloat(u32),
    IntTooLarge(u32),
}

impl LitKind {
    pub fn from_token_lit(lit: token::Lit) -> Result<LitKind, LitError> {
        let token::Lit {
            kind,
            symbol,
            suffix,
        } = lit;
    }
}
