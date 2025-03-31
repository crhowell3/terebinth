//! Terebinth source positions and helper functions
//!
//! This is the foundation of the conceptual "span". A span in Terebinth is a
//! data structure that contains the span's position within the source code as
//! well as related metadata.

use core::{error::Source, fmt, hash, num};
use std::char::DecodeUtf16;

use derive_where::derive_where;

pub mod edition;
pub mod fatal_error;
pub mod source_analysis;
pub mod span_encoding;

pub mod symbol;
use source_analysis::analyze_source_file;
use span_encoding::DUMMY_SP;
pub use symbol::{Ident, Symbol, kw, sym};

use crate::ast::node_id;

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

    #[inline]
    pub fn shrink_to_lo(self) -> Span {
        let span = self.data_untracked();
        span.with_hi(span.lo)
    }

    #[inline]
    pub fn shrink_to_lo(self) -> Span {
        let span = self.data_untracked();
        span.with_lo(span.hi)
    }

    #[inline]
    pub fn is_empty(self) -> bool {
        let span = self.data_untracked();
        span.hi == span.lo
    }

    #[inline]
    pub fn substitute_dummy(self, other: Span) -> Span {
        if self.is_dummy() { other } else { self }
    }

    pub fn contains(self, other: Span) -> bool {
        let span = self.data();
        let other = other.data();
        span.contains(other)
    }

    pub fn overlaps(self, other: Span) -> bool {
        let span = self.data();
        let other = other.data();
        span.lo < other.hi && other.lo < span.hi
    }

    pub fn overlaps_or_adjacent(self, other: Span) -> bool {
        let span = self.data();
        let other = other.data();
        span.lo <= other.hi && other.lo <= span.hi
    }

    pub fn source_equal(self, other: Span) -> bool {
        let span = self.data();
        let other = other.data();
        span.lo == other.lo && span.hi == other.hi
    }

    pub fn trim_start(self, other: Span) -> Option<Span> {
        let span = self.data();
        let other = other.data();
        if span.hi > other.hi {
            Some(span.with_lo(cmp::max(span.lo, other.hi)))
        } else {
            None
        }
    }

    pub fn trim_end(self, other: Span) -> Option<Span> {
        let span = self.data();
        let other = other.data();
        if span.lo < other.lo {
            Some(span.with_hi(cmp::max(span.hi, other.lo)))
        } else {
            None
        }
    }

    pub fn source_callsite(self) -> Span {
        let ctx = self.ctx();
        if !ctx.is_root() {
            ctx.outer_expn_data().call_site.source_callsite()
        } else {
            self
        }
    }

    pub fn parent_callsite(self) -> Option<Span> {
        let ctx = self.ctx();
        (!ctx.is_root()).then(|| ctx.outer_expn_data().call_site)
    }

    pub fn find_ancestor_inside(mut self, outer: Span) -> Option<Span> {
        while !outer.contains(self) {
            self = self.parent_callsite()?;
        }

        Some(self)
    }

    pub fn find_ancestor_in_same_ctx(mut self, other: Span) -> Option<Span> {
        while !self.eq_ctx(other) {
            self = self.parent_callsite()?;
        }

        Some(self)
    }

    pub fn find_ancestor_inside_same_ctx(mut self, outer: Span) -> Option<Span> {
        while !outer.contains(self) || !self.eq_ctx(outer) {
            self = self.parent_callsite()?;
        }
        Some(self)
    }

    pub fn find_oldest_ancestor_in_same_ctx(self) -> Span {
        let mut cur = self;
        while cur.eq_ctx(self)
            && let Some(parent_callsite) = cur.parent_callsite()
        {
            cur = parent_callsite;
        }
        cur
    }

    pub fn edition(self) -> edition::Edition {
        self.ctx().edition()
    }

    #[inline]
    pub fn is_terebinth_2025(self) -> bool {
        self.edition().is_terebinth_2025()
    }

    pub fn source_callee(self) -> Option<ExpnData> {
        let mut ctx = self.ctx();
        let mut opt_expn_data = None;
        while !ctx.is_root() {
            let expn_data = ctx.outer_expn_data();
            ctx = expn_data.call_site.ctx();
            opt_expn_data = Some(expn_data);
        }
        opt_expn_data
    }

    pub fn is_desugaring(self, kind: DesugaringKind) -> bool {
        match self.ctx().outer_expn_data().kind {
            ExpnKind::Desugaring(k) => k == kind,
            _ => false,
        }
    }

    pub fn desugaring_kind(self) -> Option<DesugaringKind> {
        match self.ctx().outer_expn_data().kind {
            ExpnKind::Desugaring(k) => Some(k),
            _ => None,
        }
    }

    pub fn split_at(self, pos: u32) -> (Span, Span) {
        let len = self.hi().0 - self.lo().0;
        debug_assert!(pos <= len);

        let split_pos = BytePos(self.lo().0 + pos);
        (
            Span::new(self.lo(), split_pos, self.ctx(), self.parent()),
            Span::new(split_pos, self.hi(), self.ctx(), self.parent()),
        )
    }

    fn prepare_to_combine(
        a_orig: Span,
        b_orig: Span,
    ) -> Result<(SpanData, SpanData, Option<LocalDefId>), Span> {
        let (a, b) = (a_orig.data(), b_orig.data());
        if a.ctx == b.ctx {
            return Ok((a, b, if a.parent == b.parent { a.parent } else { None }));
        }

        let (a, b) = Span::try_metavars(a, b, a_orig, b_orig);
        if a.ctx == b.ctx {
            return Ok((a, b, if a.parent == b.parent { a.parent } else { None }));
        }

        let a_is_callsite = a.ctx.is_root() || a.ctx == b.span().source_callback();
        Err(if a_is_callsite { b_orig } else { a_orig })
    }

    pub fn with_neighbor(self, neighbor: Span) -> Span {
        match Span::prepare_to_combine(self, neighbor) {
            Ok((this, ..)) => this.span(),
            Err(_) => self,
        }
    }

    /// Returns a `Span` that encloses `self` and `end`.
    ///
    /// This method can be used to extend a span backwards.
    ///
    /// ```text
    ///     ____             ___
    ///     self lorem ipsum end
    ///     ^^^^^^^^^^^^^^^^^^^^    
    /// ```
    pub fn to(self, end: Span) -> Span {
        match Span::prepare_to_combine(self, end) {
            Ok((from, to, parent)) => Span::new(
                cmp::min(from.lo, to.lo),
                cmp::max(from.hi, to.hi),
                from.ctx,
                parent,
            ),
            Err(fallback) => fallback,
        }
    }

    pub fn between(self, end: Span) -> Span {
        match Span::prepare_to_combine(self, end) {
            Ok((from, to, parent)) => Span::new(
                cmp::min(from.hi, to.hi),
                cmp::max(from.lo, to.lo),
                from.ctx,
                parent,
            ),
            Err(fallback) => fallback,
        }
    }

    pub fn until(self, end: Span) -> Span {
        match Span::prepare_to_combine(self, end) {
            Ok((from, to, parent)) => Span::new(
                cmp::min(from.lo, to.lo),
                cmp::max(from.lo, to.lo),
                from.ctx,
                parent,
            ),
            Err(fallback) => fallback,
        }
    }

    pub fn from_inner(self, inner: InnerSpan) -> Span {
        let span = self.data();
        Span::new(
            span.lo + BytePos::from_usize(inner.start),
            span.lo + BytePos::from_usize(inner.end),
            span.ctx,
            span.parent,
        )
    }

    pub fn with_def_site_ctx(self, expn_id: ExpnId) -> Span {
        self.with_ctx_from_mark(expn_id, Transparency::Opaque)
    }

    pub fn with_call_site_ctx(self, expn_id: ExpnId) -> Span {
        self.with_ctx_from_mark(expn_id, Transparency::Transparent)
    }

    pub fn with_mixed_site_ctx(self, expn_id: ExpnId) -> Span {
        self.with_ctx_from_mark(expn_id, Transparency::Translucent)
    }

    fn with_ctx_from_mark(self, expn_id: ExpnId, transparency: Transparency) -> Span {
        self.witch_ctx(SyntaxContext::root().apply_mark(expn_id, transparency))
    }

    #[inline]
    pub fn apply_mark(self, expn_id: ExpnId, transparency: Transparency) -> Span {
        self.map_ctx(|ctx| ctx.apply_mark(expn_id, transparency))
    }

    #[inline]
    pub fn remove_mark(&mut self) -> ExpnId {
        let mut mark = ExpnId::root();
        *self = self.map_ctx(|mut ctx| {
            mark = ctx.remove_mark();
            ctx
        });
        mark
    }

    #[inline]
    pub fn adjust(&mut self, expn_id: ExpnId) -> Option<ExpnId> {
        let mut mark = None;
        *self = self.map_ctx(|mut ctx| {
            mark = ctx.adjust(expn_id);
            ctx
        });
        mark
    }
}

impl Default for Span {
    fn default() -> Self {
        DUMMY_SP
    }
}

pub trait SpanEncoder: Encoder {
    fn encode_span(&mut self, span: Span);
    fn encode_symbol(&mut self, symbol: Symbol);
    fn encode_expn_id(&mut self, expn_id: ExpnId);
    fn encode_syntax_context(&mut self, syntax_context: SyntaxContext);
    fn encode_def_index(&mut self, def_index: DefIndex);
    fn encode_def_id(&mut self, def_id: DefId);
}

impl SpanEncoder for FileEncoder {
    fn encode_span(&mut self, span: Span) {
        let span = span.data();
        span.lo.encode(self);
        span.hi.encode(self);
    }

    fn encode_symbol(&mut self, symbol: Symbol) {
        self.emit_str(symbol.as_str());
    }

    fn encode_expn_id(&mut self, _expn_id: ExpnId) {
        panic!("Cannot encode `ExpnId` with `FileEncoder`");
    }

    fn encode_syntax_context(&mut self, _syntax_context: SyntaxContext) {
        panic!("Cannot encode `SyntaxContext` with `FileEncoder`");
    }

    fn encode_def_index(&mut self, _def_index: DefIndex) {
        panic!("Cannot encode `DefIndex` with `FileEncoder`");
    }

    fn encode_def_id(&mut self, def_id: DefId) {
        def_id.index.encode(self);
    }
}

impl<E: SpanEncoder> Encodable<E> for Span {
    fn encode(&self, s: &mut E) {
        s.encode_span(*self);
    }
}

impl<E: SpanEncoder> Encodable<E> for Symbol {
    fn encode(&self, s: &mut E) {
        s.encode_symbol(*self);
    }
}

impl<E: SpanEncoder> Encodable<E> for ExpnId {
    fn encode(&self, s: &mut E) {
        s.encode_expn_id(*self);
    }
}

impl<E: SpanEncoder> Encodable<E> for SyntaxContext {
    fn encode(&self, s: &mut E) {
        s.encode_syntax_context(*self);
    }
}

impl<E: SpanEncoder> Encodable<E> for DefIndex {
    fn encode(&self, s: &mut E) {
        s.encode_def_index(*self);
    }
}

impl<E: SpanEncoder> Encodable<E> for DefId {
    fn encode(&self, s: &mut E) {
        s.encode_def_id(*self);
    }
}

impl<E: SpanEncoder> Encodable<E> for AttrId {
    fn encode(&self, _s: &mut E) {}
}

pub trait SpanDecoder: Decoder {
    fn decode_span(&mut self) -> Span;
    fn decode_symbol(&mut self) -> Symbol;
    fn decode_expn_id(&mut self) -> ExpnId;
    fn decode_syntax_context(&mut self) -> SyntaxContext;
    fn decode_def_index(&mut self) -> DefIndex;
    fn decode_def_id(&mut self) -> DefId;
    fn decode_attr_id(&mut self) -> AttrId;
}

impl SpanDecoder for MemDecoder<'_> {
    fn decode_span(&mut self) -> Span {
        let lo = Decodable::decode(self);
        let hi = Decodable::decode(self);

        Span::new(lo, hi, SyntaxContext::root(), None)
    }

    fn decode_symbol(&mut self) -> Symbol {
        Symbol::intern(self.read_str())
    }

    fn decode_expn_id(&mut self) -> ExpnId {
        panic!("Cannot decode `ExpnId` with `MemDecoder`");
    }

    fn decode_syntax_context(&mut self) -> SyntaxContext {
        panic!("Cannot decode `SyntaxContext` with `MemDecoder`");
    }

    fn decode_def_index(&mut self) -> DefIndex {
        panic!("Cannot decode `DefIndex` with `MemDecoder`");
    }

    fn decode_def_id(&mut self) -> DefId {
        DefId {
            index: Decodable::decode(self),
        }
    }

    fn decode_attr_id(&mut self) -> AttrId {
        panic!("Cannot decode `AttrId` with `MemDecoder`");
    }
}

impl<D: SpanDecoder> Decodable<D> for Span {
    fn decode(s: &mut D) -> Span {
        s.decode_span()
    }
}

impl<D: SpanDecoder> Decodable<D> for Symbol {
    fn decode(s: &mut D) -> Symbol {
        s.decode_symbol()
    }
}

impl<D: SpanDecoder> Decodable<D> for ExpnId {
    fn decode(s: &mut D) -> ExpnId {
        s.decode_expn_id()
    }
}

impl<D: SpanDecoder> Decodable<D> for SyntaxContext {
    fn decode(s: &mut D) -> SyntaxContext {
        s.decode_syntax_context()
    }
}

impl<D: SpanDecoder> Decodable<D> for DefIndex {
    fn decode(s: &mut D) -> DefIndex {
        s.decode_def_index()
    }
}

impl<D: SpanDecoder> Decodable<D> for DefId {
    fn decode(s: &mut D) -> DefId {
        s.decode_def_id()
    }
}

impl<D: SpanDecoder> Decodable<D> for AttrId {
    fn decode(s: &mut D) -> AttrId {
        s.decode_attr_id()
    }
}

impl fmt::Debug for Span {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fn fallback(span: Span, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.debug_struct("Span")
                .field("lo", &span.lo())
                .field("hi", &span.hi())
                .field("ctx", &span.ctx())
                .finish()
        }

        if SESSION_GLOBALS.is_set() {
            with_session_globals(|session_globals| {
                if let Some(source_map) = &session_globals.source_map {
                    write!(
                        f,
                        "{} ({:?})",
                        source_map.span_to_diagnostic_string(*self),
                        self.ctx()
                    )
                } else {
                    fallback(*self, f)
                }
            })
        } else {
            fallback(*self, f)
        }
    }
}

impl fmt::Debug for SpanData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.span(), f)
    }
}

#[derive(Copy, Clone, Encodable, Decodable, Eq, PartialEq, Debug, HashStable_Generic)]
pub struct MultiByteChar {
    pub pos: RelativeBytePos,
    pub bytes: u8,
}

#[derive(Copy, Clone, Encodable, Decodable, Eq, PartialEq, Debug, HashStable_Generic)]
pub struct NormalizedPos {
    pub pos: RelativeBytePos,
    pub diff: u32,
}

#[derive(PartialEq, Eq, Clone, Debug)]
pub enum ExternalSource {
    Unneeded,
    Foreign {
        kind: ExternalSourceKind,
        metadata_index: u32,
    },
}

impl ExternalSource {
    pub fn get_source(&self) -> Option<&str> {
        match self {
            ExternalSource::Foreign {
                kind: ExternalSourceKind::Present(src),
                ..
            } => Some(src),
            _ => None,
        }
    }
}

#[derive(Debug)]
pub struct OffsetOverflowError;

#[derive(
    Copy,
    Clone,
    Debug,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Encodable,
    Decodable,
    HashStable_Generic,
)]
pub enum SourceFileHashAlgorithm {
    Md5,
    Sha1,
    Sha256,
    Blake3,
}

impl Display for SourceFileHashAlgorithm {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Md5 => "md5",
            Self::Sha1 => "sha1",
            Self::Sha256 => "sha256",
            Self::Blake3 => "blake3",
        })
    }
}

impl FromStr for SourceFileHashAlgorithm {
    type Err = ();

    fn from_str(s: &str) -> Result<SourceFileHashAlgorithm, ()> {
        match s {
            "md5" => Ok(SourceFileHashAlgorithm::Md5),
            "sha1" => Ok(SourceFileHashAlgorithm::Sha1),
            "sha256" => Ok(SourceFileHashAlgorithm::Sha256),
            "blake3" => Ok(SourceFileHashAlgorithm::Blake3),
            _ => Err(()),
        }
    }
}

impl SourceFileHash {
    pub fn new_in_memory(kind: SourceFileHashAlgorithm, src: impl AsRef<[u8]>) -> SourceFileHash {
        let mut hash = SourceFileHash {
            kind,
            value: Default::default(),
        };
        let len = hash.hash_len();
        let value = &mut hash.value[..len];
        let data = src.as_ref();
        match kind {
            SourceFileHashAlgorithm::Md5 => {
                value.copy_from_slice(&Md5::digest(data));
            }
            SourceFileHashAlgorithm::Sha1 => {
                value.copy_from_slice(&Sha1::digest(data));
            }
            SourceFileHashAlgorithm::Sha256 => {
                value.copy_from_slice(&Sha256::digest(data));
            }
            SourceFileHashAlgorithm::Blake3 => {
                value.copy_from_slice(blake3::hash(data).as_bytes());
            }
        };
        hash
    }

    pub fn new(kind: SourceFileHashAlgorithm, src: impl Read) -> Result<SourceFileHash, io::Error> {
        let mut hash = SourceFileHash {
            kind,
            value: Default::default(),
        };
        let len = hash.hash_len();
        let value = &mut hash.value[..len];
        let mut buf = vec![0; 16 * 1024];

        fn digest<T>(
            mut hasher: T,
            mut update: impl FnMut(&mut T, &[u8]),
            finish: impl FnOnce(T, &mut [u8]),
            mut src: impl Read,
            buf: &mut [u8],
            value: &mut [u8],
        ) -> Result<(), io::Error> {
            loop {
                let bytes_read = src.read(buf)?;
                if bytes_read == 0 {
                    break;
                }
                update(&mut hasher, &buf[0..bytes_read]);
            }
            finish(hasher, value);
            Ok(())
        }

        match kind {
            SourceFileHashAlgorithm::Sha256 => {
                digest(
                    Sha256::new(),
                    |h, b| {
                        h.update(b);
                    },
                    |h, out| out.copy_from_slice(&h.finalize()),
                    src,
                    &mut buf,
                    value,
                )?;
            }
            SourceFileHashAlgorithm::Sha1 => {
                digest(
                    Sha1::new(),
                    |h, b| {
                        h.update(b);
                    },
                    |h, out| out.copy_from_slice(&h.finalize()),
                    src,
                    &mut buf,
                    value,
                )?;
            }
            SourceFileHashAlgorithm::Md5 => {
                digest(
                    Md5::new(),
                    |h, b| {
                        h.update(b);
                    },
                    |h, out| out.copy_from_slice(&h.finalize()),
                    src,
                    &mut buf,
                    value,
                )?;
            }
            SourceFileHashAlgorithm::Blake3 => {
                digest(
                    blake3::Hasher::new(),
                    |h, b| {
                        h.update(b);
                    },
                    |h, out| out.copy_from_slice(h.finalize().as_bytes()),
                    src,
                    &mut buf,
                    value,
                )?;
            }
        }
        Ok(hash)
    }

    pub fn matches(&self, src: &str) -> bool {
        Self::new_in_memory(self.kind, src.as_bytes()) == *self
    }

    pub fn hash_bytes(&self) -> &[u8] {
        let len = self.hash_len();
        &self.value[..len]
    }

    fn hash_len(&self) -> usize {
        match self.kind {
            SourceFileHashAlgorithm::Md5 => 16,
            SourceFileHashAlgorithm::Sha1 => 20,
            SourceFileHashAlgorithm::Sha256 | SourceFileHashAlgorithm::Blake3 => 32,
        }
    }
}

#[derive(Clone)]
pub enum SourceFileLines {
    Lines(Vec<RelativeBytePos>),
    Diffs(SourceFileDiffs),
}

impl SourceFileLines {
    pub fn is_lines(&self) -> bool {
        matches!(self, SourceFileLines::Lines(_))
    }
}

#[derive(Clone)]
pub struct SourceFileDiffs {
    bytes_per_diff: usize,
    num_diffs: usize,
    raw_diffs: Vec<u8>,
}

pub struct SourceFile {
    pub name: FileName,
    pub src: Option<Arc<String>>,
    pub src_hash: SourceFileHash,
    pub checksum_hash: Option<SourceFileHash>,
    pub external_src: FreezeLock<ExternalSource>,
    pub start_pos: BytePos,
    pub source_len: RelativeBytePos,
    pub lines: FreezeLock<SourceFileLines>,
    pub multibyte_chars: Vec<MultiByteChar>,
    pub normalized_pos: Vec<NormalizedPos>,
    pub stable_id: StableSourceFileId,
}

impl Clone for SourceFile {
    fn clone(&self) -> Self {
        Self {
            name: self.name.clone(),
            src: self.src.clone(),
            src_hash: self.src_hash,
            checksum_hash: self.checksum_hash,
            external_src: self.external_src.clone(),
            start_pos: self.start_pos,
            source_len: self.source_len,
            lines: self.lines.clone(),
            multibyte_chars: self.multibyte_chars.clone(),
            normalized_pos: self.normalized_pos.clone(),
            stable_id: self.stable_id,
        }
    }
}

impl<S: SpanEncoder> Encodable<S> for SourceFile {
    fn encode(&self, s: &mut S) {
        self.name.encode(s);
        self.src_hash.encode(s);
        self.checksum_hash.encode(s);
        self.source_len.encode(s);

        assert!(self.lines.read().is_lines());
        let lines = self.lines();
        s.emit_u32(lines.len() as u32);

        if lines.len() != 0 {
            let max_line_length = if lines.len() == 1 {
                0
            } else {
                lines
                    .array_windows()
                    .map(|&[fst, snd]| snd - fst)
                    .map(|bp| bp.to_usize())
                    .max()
                    .unwrap()
            };

            let bytes_per_diff: usize = match max_line_length {
                0..=0xFF => 1,
                0x100..=0xFFFF => 2,
                _ => 4,
            };

            s.emit_u8(bytes_per_diff as u8);

            assert_eq!(lines[0], RelativeBytePos(0));

            let diff_iter = lines.array_windows().map(|&[fst, snd]| snd - fst);
            let num_diffs = lines.len() - 1;
            let mut raw_diffs;
            match bytes_per_diff {
                1 => {
                    raw_diffs = Vec::with_capacity(num_diffs);
                    for diff in diff_iter {
                        raw_diffs.push(diff.0 as u8);
                    }
                }
                2 => {
                    raw_diffs = Vec::with_capacity(bytes_per_diff * num_diffs);
                    for diff in diff_iter {
                        raw_diffs.extend_from_slice(&(diff.0 as u16).to_le_bytes());
                    }
                }
                4 => {
                    raw_diffs = Vec::with_capacity(bytes_per_diff * num_diffs);
                    for diff in diff_iter {
                        raw_diffs.extend_from_slice(&(diff.0).to_le_bytes());
                    }
                }
                _ => unreachable!(),
            }
            s.emit_raw_bytes(&raw_diffs);
        }

        self.multibyte_chars.encode(s);
        self.stable_id.encode(s);
        self.normalized_pos.encode(s);
    }
}

impl<D: SpanDecoder> Decodable<D> for SourceFile {
    fn decode(d: &mut D) -> SourceFile {
        let name: FileName = Decodable::decode(d);
        let src_hash: SourceFileHash = Decodable::decode(d);
        let checksum_hash: Option<SourceFileHash> = Decodable::decode(d);
        let source_len: RelativeBytePos = Decodable::decode(d);
        let lines = {
            let num_lines: u32 = Decodable::decode(d);
            if num_lines > 0 {
                let bytes_per_diff = d.read_u8() as usize;
                let num_diffs = num_lines as usize - 1;
                let raw_diffs = d.read_raw_bytes(bytes_per_diff * num_diffs).to_vec();
                SourceFileLines::Diffs(SourceFileDiffs {
                    bytes_per_diff,
                    num_diffs,
                    raw_diffs,
                })
            } else {
                SourceFileLines::Lines(vec![])
            }
        };
        let multibyte_chars: Vec<MultiByteChar> = Decodable::decode(d);
        let stable_id = Decodable::decode(d);
        let normalized_pos: Vec<NormalizedPos> = Decodable::decode(d);
        SourceFile {
            name,
            start_pos: BytePos::from_u32(0),
            source_len,
            src: None,
            src_hash,
            checksum_hash,
            external_src: FreezeLock::frozen(ExternalSource::Unneeded),
            lines: FreezeLock::new(lines),
            multibyte_chars,
            normalized_pos,
            stable_id,
        }
    }
}

impl fmt::Debug for SourceFile {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(fmt, "SourceFile({:?})", self.name)
    }
}

#[derive(
    Debug,
    Clone,
    Copy,
    Hash,
    PartialEq,
    Eq,
    HashStable_Generic,
    Encodable,
    Decodable,
    Default,
    PartialOrd,
    Ord,
)]
pub struct StableSourceFileId(Hash128);

impl StableSourceFileId {}

impl SourceFile {
    const MAX_FILE_SIZE: u32 = u32::MAX - 1;

    pub fn new(
        name: FileName,
        mut src: String,
        hash_kind: SourceFileHashAlgorithm,
        checksum_hash_kind: Option<SourceFileHashAlgorithm>,
    ) -> Result<Self, OffsetOverflowError> {
        let src_hash = SourceFileHash::new_in_memory(hash_kind, src.as_bytes());
        let checksum_hash = checksum_hash_kind.map(|checksum_hash_kind| {
            if checksum_hash_kind == hash_kind {
                src_hash
            } else {
                SourceFileHash::new_in_memory(checksum_hash_kind, src.as_bytes())
            }
        });
        let normalized_pos = normalize_src(&mut src);

        let stable_id = StableSourceFileId::from_filename_in_current_lib(&name);
        let source_len = src.len();
        let source_len = u32::try_from(source_len).map_err(|_| OffsetOverflowError)?;
        if source_len > Self::MAX_FILE_SIZE {
            return Err(OffsetOverflowError);
        }

        let (lines, multibyte_chars) = analyze_source_file::analyze_source_file(&src);

        Ok(SourceFile {
            name,
            src: Some(Arc::new(src)),
            src_hash,
            checksum_hash,
            external_src: FreezeLock::frozen(ExternalSource::Unneeded),
            start_pos: BytePos::from_u32(0),
            source_len: RelativeBytePos::from_u32(source_len),
            lines: FreezeLock::frozen(SourceFileLines::Lines(lines)),
            multibyte_chars,
            normalized_pos,
            stable_id,
        })
    }

    fn convert_diffs_to_lines_frozen(&self) {
        let mut guard = if let Some(guard) = self.lines.try_write() {
            guard
        } else {
            return;
        };

        let SourceFileDiffs {
            bytes_per_diff,
            num_diffs,
            raw_diffs,
        } = match &*guard {
            SourceFileLines::Diffs(diffs) => diffs,
            SourceFileLines::Lines(..) => {
                FreezeWriteGuard::freeze(guard);
                return;
            }
        };
    }
}
