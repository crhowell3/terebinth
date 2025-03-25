//! Terebinth source positions and helper functions
//!
//! This is the foundation of the conceptual "span". A span in Terebinth is a
//! data structure that contains the span's position within the source code as
//! well as related metadata.

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

scoped_tls::scoped_thread_local(static SESSION_GLOBALS: SessionGlobals);
