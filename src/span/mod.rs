//! Terebinth source positions and helper functions
//!
//! This is the foundation of the conceptual "span". A span in Terebinth is a
//! data structure that contains the span's position within the source code as
//! well as related metadata.

use derive_where::derive_where;

pub mod edition;
pub mod fatal_error;
pub mod source_analysis;
pub mod span_encoding;

pub mod symbol;
pub use symbol::{Ident, Symbol, kw, sym};

pub struct SessionGlobals {
    symbol_interner: symbol::Interner,
    span_interner: Lock<span_encoding::SpanInterner>,
    metavar_spans: MetavarSpansMap,
    hygiene_data: Lock<hygiene::HygieneData>,
    source_map: Option<Arc<SourceMap>>,
}

impl SessionGlobals {
    pub fn new(edition: Edition, sm_inputs: Option<SourceMapInputs>) -> SessionGlobals {
        SessionGlobals {
            symbol_interner: symbol::Interner::fresh(),
            span_interner: Lock::new(span_encoding::SpanInterner::default()),
            metavar_spans: Default::default(),
            hygiene_data: Lock::new(hygiene::HygieneData::new(edition)),
            source_map: sm_inputs.map(|inputs| Arc::new(SourceMap::with_inputs(inputs))),
        }
    }
}

pub fn create_session_globals_then<R>(
    edition: Edition,
    sm_inputs: Option<SourceMapInputs>,
    f: impl FnOnce() -> R,
) -> R {
    assert!(
        !SESSION_GLOBALS.is_set(),
        "SESSION_GLOBALS should never be overwritten! \
         Use another thread if you need another SessionGlobals"
    );
    let session_globals = SessionGlobals::new(edition, sm_inputs);
    SESSION_GLOBALS.set(&session_globals, f)
}

pub fn set_session_globals_then<R>(session_globals: &SessionGlobals, f: impl FnOnce() -> R) -> R {
    assert!(
        !SESSION_GLOBALS.is_set(),
        "SESSION_GLOBALS should never be overwritten! \
         Use another thread if you need another SessionGlobals"
    );
    SESSION_GLOBALS.set(session_globals, f)
}

pub fn create_session_if_not_set_then<R, F>(edition: Edition, f: F) -> R
where
    F: FnOnce(&SessionGlobals) -> R,
{
    if !SESSION_GLOBALS.is_set() {
        let session_globals = SessionGlobals::new(edition, None);
        SESSION_GLOBALS.set(&session_globals, || SESSION_GLOBALS.with(f))
    } else {
        SESSION_GLOBALS.with(f)
    }
}

pub fn with_session_globals<R, F>(f: F) -> R
where
    F: FnOnce(&SessionGlobals) -> R,
{
    SESSION_GLOBALS.with(f)
}

pub fn create_default_session_globals_then<R>(f: impl FnOnce() -> R) -> R {
    create_session_globals_then(edition::DEFAULT_EDITION, None, f)
}

scoped_tls::scoped_thread_local!(static SESSION_GLOBALS: SessionGlobals);

#[derive(Clone, Copy, Hash, PartialEq, Eq)]
#[derive_where(PartialOrd, Ord)]
pub struct SpanData {
    pub lo: BytePos,
    pub hi: BytePos,
    #[derive_where(skip)]
    pub ctx: SyntaxContext,
    #[derive_where(skip)]
    pub parent: Option<LocalDefId>,
}

impl SpanData {
    #[inline]
    pub fn span(&self) -> Span {
        Span::new(self.lo, self.hi, self.ctx, self.parent)
    }

    #[inline]
    pub fn with_lo(&self, lo: BytePos) -> Span {
        Span::new(lo, self.hi, self.ctx, self.parent)
    }

    #[inline]
    pub fn with_hi(&self, hi: BytePos) -> Span {
        Span::new(self.lo, hi, self.ctx, self.parent)
    }

    #[inline]
    pub fn with_ctx(&self, ctx: SyntaxContext) -> Span {
        Span::new(self.lo, self.hi, ctx, self.parent)
    }

    #[inline]
    pub fn with_parent(&self, parent: Option<LocalDefId>) -> Span {
        Span::new(self.lo, self.hi, self.ctx, parent)
    }

    #[inline]
    pub fn is_dummy(self) -> bool {
        self.lo.0 == 0 && self.hi.0 == 0
    }

    pub fn contains(self, other: Self) -> bool {
        self.lo <= other.lo && other.hi <= self.hi
    }
}

impl Default for SpanData {
    fn default() -> Self {
        Self {
            lo: BytePos(0),
            hi: BytePos(0),
            ctx: SyntaxContext::root(),
            parent: None,
        }
    }
}

impl PartialOrd for Span {
    fn partial_cmp(&self, rhs: &Self) -> Option<Ordering> {
        PartialOrd::partial_cmp(&self.data(), &rhs.data())
    }
}

impl Ord for Span {
    fn cmp(&self, rhs: &Self) -> Ordering {
        Ord::cmp(&self.data(), &rhs.data())
    }
}

impl Span {
    #[inline]
    pub fn lo(self) -> BytePos {
        self.data().lo
    }

    #[inline]
    pub fn with_lo(self, lo: BytePos) -> Span {
        self.data().with_lo(lo)
    }

    #[inline]
    pub fn hi(self) -> BytePos {
        self.data().hi
    }

    #[inline]
    pub fn with_hi(self, hi: BytePos) -> Span {
        self.data().with_hi(hi)
    }

    #[inline]
    pub fn with_ctx(self, ctx: SyntaxContext) -> Span {
        self.map_ctx(|_| ctx)
    }

    #[inline]
    pub fn is_visible(self, sm: &SourceMap) -> bool {
        !self.is_dummy() && sm.is_span_accessible(self)
    }

    #[inline]
    pub fn with_root_ctx(lo: BytePos, hi: BytePos) -> Span {
        Span::new(lo, hi, SyntaxContext::root(), None)
    }
}
