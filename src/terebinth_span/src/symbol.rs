//! The Terebinth symbol interner. This allows for bidirectional lookup, meaning
//! that a type can be used to look up a value, and a value can be used to look
//! up a type.

use std::hash::{Hash, Hasher};
use std::{fmt, str};

use terebinth_arena_allocator::DroplessArena;
use terebinth_data_structures::fx::FxIndexSet;
use terebinth_data_structures::stable_hasher::{
    HashStable, StableCompare, StableHasher, ToStableHashKey,
};
use terebinth_data_structures::sync::Lock;
use terebinth_macros::{Decodable, Encodable, HashStable_Generic, symbols};

use crate::{DUMMY_SP, Edition, Span, with_session_globals};

symbols! {
    Keywords {
        Empty: "",
        Underscore: "_",

        // Terebinth keywords
        Break: "break",
        Const: "const",
        Continue: "continue",
        Else: "else",
        Enum: "enum",
        False: "false",
        Func: "func",
        For: "for",
        If: "if",
        Let: "let",
        Return: "return",
        Static: "static",
        Struct: "struct",
        True: "true",
        Type: "type",
        While: "while",
    }
    Symbols {
        Break,
        // TODO
    }
}

#[derive(Copy, Clone, Eq, HashStable_Generic, Encodable, Decodable)]
pub struct Ident {
    pub name: Symbol,
    pub span: Span,
}

impl Ident {
    #[inline]
    pub const fn new(name: Symbol, span: Span) -> Ident {
        Ident { name, span }
    }

    #[inline]
    pub const fn with_dummy_span(name: Symbol) -> Ident {
        Ident::new(name, DUMMY_SP)
    }

    #[inline]
    pub fn empty() -> Ident {
        Ident::with_dummy_span(kw::Empty)
    }

    pub fn from_str(string: &str) -> Ident {
        Ident::with_dummy_span(Symbol::intern(string))
    }

    pub fn from_str_and_span(string: &str, span: Span) -> Ident {
        Ident::new(Symbol::intern(string), span)
    }

    pub fn with_span_pos(self, span: Span) -> Ident {
        Ident::new(self.name, span.with_ctx(self.span.ctx()))
    }

    pub fn as_str(&self) -> &str {
        self.name.as_str()
    }
}

impl PartialEq for Ident {
    #[inline]
    fn eq(&self, rhs: &Self) -> bool {
        self.name == rhs.name && self.span.eq_ctx(rhs.span)
    }
}

impl Hash for Ident {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.name.hash(state);
        self.span.ctx().hash(state);
    }
}

impl fmt::Debug for Ident {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&IdentPrinter::new(self.name, self.is_raw_guess()), f)
    }
}

pub struct IdentPrinter {
    symbol: Symbol,
    is_raw: bool,
}

impl IdentPrinter {
    pub fn new(symbol: Symbol, is_raw: bool) -> IdentPrinter {
        IdentPrinter { symbol, is_raw }
    }

    pub fn for_ast_ident(ident: Ident, is_raw: bool) -> IdentPrinter {
        IdentPrinter::new(ident.name, is_raw)
    }
}

impl fmt::Display for IdentPrinter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_raw {
            f.write_str("r#")?;
        }
        fmt::Display::fmt(&self.symbol, f)
    }
}

/// The `Symbol` is an interned string.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Symbol(SymbolIndex);

index::newtype_index! {
    #[orderable]
    struct SymbolIndex{}
}

impl Symbol {
    const fn new(n: u32) -> Self {
        Symbol(SymbolIndex::from_u32(n))
    }

    pub fn new_from_decoded(n: u32) -> Self {
        Self::new(n)
    }

    pub fn intern(string: &str) -> Self {
        with_session_globals(|session_globals| session_globals.symbol_interner.intern(string))
    }

    pub fn as_str(&self) -> &str {
        with_session_globals(|session_globals| unsafe {
            std::mem::transmute::<&str, &str>(session_globals.symbol_interner.get(*self))
        })
    }

    pub fn as_u32(self) -> u32 {
        self.0.as_u32()
    }

    pub fn is_empty(self) -> bool {
        self == kw::Empty
    }

    pub fn to_ident_string(self) -> String {
        Ident::with_dummy_span(self).to_string()
    }
}

impl fmt::Debug for Symbol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self.as_str(), f)
    }
}

impl fmt::Display for Symbol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self.as_str(), f)
    }
}

impl<CTX> HashStable<CTX> for Symbol {
    #[inline]
    fn hash_stable(&self, hcx: &mut CTX, hasher: &mut StableHasher) {
        self.as_str().hash_stable(hcx, hasher);
    }
}

impl<CTX> ToStableHashKey<CTX> for Symbol {
    type KeyType = String;
    #[inline]
    fn to_stable_hash_key(&self, _: &CTX) -> String {
        self.as_str().to_string()
    }
}

impl StableCompare for Symbol {
    const CAN_USE_UNSTABLE_SORT: bool = true;

    fn stable_cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.as_str().cmp(other.as_str())
    }
}

pub(crate) struct Interner(Lock<InternerInner>);

struct InternerInner {
    arena: DroplessArena,
    strings: FxIndexSet<&'static str>,
}

impl Interner {
    fn prefill(init: &[&'static str]) -> Self {
        Interner(Lock::new(InternerInner {
            arena: Default::default(),
            strings: init.iter().copied().collect(),
        }))
    }

    #[inline]
    fn intern(&self, string: &str) -> Symbol {
        let mut inner = self.0.lock();
        if let Some(idx) = inner.strings.get_index_of(string) {
            return Symbol::new(idx as u32);
        }

        let string: &str = inner.arena.alloc_str(string);

        let string: &'static str = unsafe { &*(string as *const str) };

        let (idx, is_new) = inner.strings.insert_full(string);
        debug_assert!(is_new);

        Symbol::new(idx as u32)
    }

    fn get(&self, symbol: Symbol) -> &str {
        self.0
            .lock()
            .strings
            .get_index(symbol.0.as_usize())
            .unwrap()
    }
}

pub mod kw {
    pub use super::kw_generated::*;
}

pub mod sym {
    use super::Symbol;
    pub use super::kw::MacroRules as macro_rules;
    #[doc(inline)]
    pub use super::sym_generated::*;

    pub fn integer<N: TryInto<usize> + Copy + itoa::Integer>(n: N) -> Symbol {
        if let Result::Ok(idx) = n.try_into() {
            if idx < 10 {
                return Symbol::new(super::SYMBOL_DIGITS_BASE + idx as u32);
            }
        }
        let mut buffer = itoa::Buffer::new();
        let printed = buffer.format(n);
        Symbol::intern(printed)
    }
}

impl Symbol {
    fn is_special(self) -> bool {
        self <= kw::Underscore
    }

    fn is_used_keyword_always(self) -> bool {
        self >= kw::As && self <= kw::While
    }

    pub fn is_reserved(self, edition: impl Copy + FnOnce() -> Edition) -> bool {
        self.is_special || self.is_used_keyword_always()
    }

    pub fn is_bool_lit(self) -> bool {
        self == kw::True || self == kw::False
    }

    pub fn is_preinterned(self) -> bool {
        self.as_u32() < PREINTERNED_SYMBOLS_COUNT
    }
}

impl Ident {
    pub fn is_special(self) -> bool {
        self.name.is_special()
    }

    pub fn is_used_keyword(self) -> bool {
        self.name.is_used_keyword_always()
    }

    pub fn is_reserved(self) -> bool {
        self.name.is_reserved(|| self.span.edition())
    }

    pub fn is_numeric(self) -> bool {
        !self.name.is_empty() && self.as_str().bytes().all(|b| b.is_ascii_digit())
    }
}

pub fn used_keywords(edition: impl Copy + FnOnce() -> Edition) -> Vec<Symbol> {
    (kw::Empty.as_u32()..kw::While.as_u32())
        .filter_map(|kw| {
            let kw = Symbol::new(kw);
            if kw.is_used_keyword_always() {
                Some(kw)
            } else {
                None
            }
        })
        .collect()
}
