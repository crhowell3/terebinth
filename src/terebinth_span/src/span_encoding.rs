use terebinth_data_structures::fx::FxIndexSet;
use terebinth_serialize::int_overflow::DebugStrictAdd;

use crate::def_id::{DefIndex, LocalDefId};
use crate::hygiene::SyntaxContext;
use crate::{BytePos, SPAN_TRACK, SpanData};

#[derive(Clone, Copy, Eq, PartialEq, Hash)]
pub struct Span {
    lo_or_index: u32,
    len_with_tag_or_marker: u16,
    ctx_or_parent_or_marker: u16,
}

#[derive(Clone, Copy)]
struct InlineCtx {
    lo: u32,
    len: u16,
    ctx: u16,
}

#[derive(Clone, Copy)]
struct InlineParent {
    lo: u32,
    len_with_tag: u16,
    parent: u16,
}

#[derive(Clone, Copy)]
struct PartiallyInterned {
    index: u32,
    ctx: u16,
}

#[derive(Clone, Copy)]
struct Interned {
    index: u32,
}

impl InlineCtx {
    #[inline]
    fn data(self) -> SpanData {
        let len = self.len as u32;
        debug_assert!(len <= MAX_LEN);
        SpanData {
            lo: BytePos(self.lo),
            hi: BytePos(self.lo.debug_strict_add(len)),
            ctx: SyntaxContext::from_u16(self.ctx),
            parent: None,
        }
    }

    #[inline]
    fn span(lo: u32, len: u16, ctx: u16) -> Span {
        Span {
            lo_or_index: lo,
            len_with_tag_or_marker: len,
            ctx_or_parent_or_marker: ctx,
        }
    }

    #[inline]
    fn from_span(span: Span) -> InlineCtx {
        let (lo, len, ctx) = (
            span.lo_or_index,
            span.len_with_tag_or_marker,
            span.ctx_or_parent_or_marker,
        );
        InlineCtx { lo, len, ctx }
    }
}

impl InlineParent {
    #[inline]
    fn data(self) -> SpanData {
        let len = (self.len_with_tag & !PARENT_TAG) as u32;
        debug_assert!(len <= MAX_LEN);
        SpanData {
            lo: BytePos(self.lo),
            hi: BytePos(self.lo.debug_strict_add(len)),
            ctx: SyntaxContext::root(),
            parent: Some(LocalDefId {
                local_def_index: DefIndex::from_u16(self.parent),
            }),
        }
    }

    #[inline]
    fn span(lo: u32, len: u16, parent: u16) -> Span {
        let (lo_or_index, len_with_tag_or_marker, ctx_or_parent_or_marker) =
            (lo, PARENT_TAG | len, parent);
        Span {
            lo_or_index,
            len_with_tag_or_marker,
            ctx_or_parent_or_marker,
        }
    }

    #[inline]
    fn from_span(span: Span) -> InlineParent {
        let (lo, len_with_tag, parent) = (
            span.lo_or_index,
            span.len_with_tag_or_marker,
            span.ctx_or_parent_or_marker,
        );
        InlineParent {
            lo,
            len_with_tag,
            parent,
        }
    }
}

impl PartiallyInterned {
    #[inline]
    fn data(self) -> SpanData {
        SpanData {
            ctx: SyntaxContext::from_u16(self.ctx),
            ..with_span_interner(|interner| interner.spans[self.index as usize])
        }
    }

    #[inline]
    fn span(index: u32, ctx: u16) -> Span {
        let (lo_or_index, len_with_tag_or_marker, ctx_or_parent_or_marker) =
            (index, BASE_LEN_INTERNED_MARKER, ctx);
        Span {
            lo_or_index,
            len_with_tag_or_marker,
            ctx_or_parent_or_marker,
        }
    }
    #[inline]
    fn from_span(span: Span) -> PartiallyInterned {
        PartiallyInterned {
            index: span.lo_or_index,
            ctx: span.ctx_or_parent_or_marker,
        }
    }
}

impl Interned {
    #[inline]
    fn data(self) -> SpanData {
        with_span_interner(|interner| interner.spans[self.index as usize])
    }
    #[inline]
    fn span(index: u32) -> Span {
        let (lo_or_index, len_with_tag_or_marker, ctx_or_parent_or_marker) =
            (index, BASE_LEN_INTERNED_MARKER, CTX_INTERNED_MARKER);
        Span {
            lo_or_index,
            len_with_tag_or_marker,
            ctx_or_parent_or_marker,
        }
    }
    #[inline]
    fn from_span(span: Span) -> Interned {
        Interned {
            index: span.lo_or_index,
        }
    }
}

macro_rules! match_span_kind {
    (
        $span:expr,
        InlineCtx($span1:ident) => $arm1:expr,
        InlineParent($span2:ident) => $arm2:expr,
        PartiallyInterned($span3:ident) => $arm3:expr,
        Interned($span4:ident) => $arm4:expr,
    ) => {
        if $span.len_with_tag_or_marker != BASE_LEN_INTERNED_MARKER {
            if $span.len_with_tag_or_marker & PARENT_TAG == 0 {
                let $span1 = InlineCtx::from_span($span);
                $arm1
            } else {
                let $span2 = InlineParent::from_span($span);
                $arm2
            }
        } else if $span.ctx_or_parent_or_marker != CTX_INTERNED_MARKER {
            let $span3 = PartiallyInterned::from_span($span);
            $arm3
        } else {
            let $span4 = Interned::from_span($span);
            $arm4
        }
    };
}

const MAX_LEN: u32 = 0b0111_1111_1111_1110;
const MAX_CTX: u32 = 0b0111_1111_1111_1110;
const PARENT_TAG: u16 = 0b1000_0000_0000_0000;
const BASE_LEN_INTERNED_MARKER: u16 = 0b1111_1111_1111_1111;
const CTX_INTERNED_MARKER: u16 = 0b1111_1111_1111_1111;

pub const DUMMY_SP: Span = Span {
    lo_or_index: 0,
    len_with_tag_or_marker: 0,
    ctx_or_parent_or_marker: 0,
};

impl Span {
    #[inline]
    pub fn new(
        mut lo: BytePos,
        mut hi: BytePos,
        ctx: SyntaxContext,
        parent: Option<LocalDefId>,
    ) -> Self {
        if lo > hi {
            std::mem::swap(&mut lo, &mut hi);
        }

        let (len, ctx32) = (hi.0 - lo.0, ctx.as_u32());
        if len <= MAX_LEN && ctx32 <= MAX_CTX {
            match parent {
                None => return InlineCtx::span(lo.0, len as u16, ctx32 as u16),
                Some(parent) => {
                    let parent32 = parent.local_def_index.as_u32();
                    if ctx32 == 0 && parent32 <= MAX_CTX {
                        return InlineParent::span(lo.0, len as u16, parent32 as u16);
                    }
                }
            }
        }

        let index = |ctx| {
            with_span_interner(|interner| {
                interner.intern(&SpanData {
                    lo,
                    hi,
                    ctx,
                    parent,
                })
            })
        };
        if ctx32 <= MAX_CTX {
            PartiallyInterned::span(index(SyntaxContext::from_u32(u32::MAX)), ctx32 as u16)
        } else {
            Interned::span(index(ctx))
        }
    }

    #[inline]
    pub fn data(self) -> SpanData {
        let data = self.data_untracked();
        if let Some(parent) = data.parent {
            (*SPAN_TRACK)(parent);
        }
        data
    }

    #[inline]
    pub fn data_untracked(self) -> SpanData {
        match_span_kind! {
            self,
            InlineCtx(span) => span.data(),
            InlineParent(span) => span.data(),
            PartiallyInterned(span) => span.data(),
            Interned(span) => span.data(),
        }
    }

    #[inline]
    pub fn from_expansion(self) -> bool {
        self.inline_ctx().map_or(true, |ctx| !ctx.is_root())
    }

    #[inline]
    pub fn is_dummy(self) -> bool {
        if self.len_with_tag_or_marker != BASE_LEN_INTERNED_MARKER {
            let lo = self.lo_or_index;
            let len = (self.len_with_tag_or_marker & !PARENT_TAG) as u32;
            debug_assert!(len <= MAX_LEN);
            lo == 0 && len == 0
        } else {
            let index = self.lo_or_index;
            let data = with_span_interner(|interner| interner.spans[index as usize]);
            data.lo == BytePos(0) && data.hi == BytePos(0)
        }
    }

    #[inline]
    pub fn map_ctx(self, map: impl FnOnce(SyntaxContext) -> SyntaxContext) -> Span {
        let data = match_span_kind! {
            self,
            InlineCtx(span) => {
                let new_ctx = map(SyntaxContext::from_u16(span.ctx));
                let new_ctx32 = new_ctx.as_u32();
                return if new_ctx32 <= MAX_CTX {
                    InlineCtx::span(span.lo, span.len, new_ctx32 as u16)
                } else {
                    span.data().with_ctx(new_ctx)
                };
            },
            InlineParent(span) => span.data(),
            PartiallyInterned(span) => span.data(),
            Interned(span) => span.data(),
        };

        data.with_ctx(map(data.ctx))
    }

    #[inline]
    fn inline_ctx(self) -> Result<SyntaxContext, usize> {
        match_span_kind! {
            self,
            InlineCtx(span) => Ok(SyntaxContext::from_u16(span.ctx)),
            InlineParent(_span) => Ok(SyntaxContext::root()),
            PartiallyInterned(span) => Ok(SyntaxContext::from_u16(span.ctx)),
            Interned(span) => Err(span.index as usize),
        }
    }

    #[cfg_attr(not(test), rustc_diagnostic_item = "SpanCtx")]
    #[inline]
    pub fn ctx(self) -> SyntaxContext {
        self.inline_ctx()
            .unwrap_or_else(|index| with_span_interner(|interner| interner.spans[index].ctx))
    }

    #[inline]
    pub fn eq_ctx(self, other: Span) -> bool {
        match (self.inline_ctx(), other.inline_ctx()) {
            (Ok(ctx1), Ok(ctx2)) => ctx1 == ctx2,
            // If `inline_ctx` returns `Ok` the context is <= MAX_CTXT.
            // If it returns `Err` the span is fully interned and the context is > MAX_CTXT.
            // As these do not overlap an `Ok` and `Err` result cannot have an equal context.
            (Ok(_), Err(_)) | (Err(_), Ok(_)) => false,
            (Err(index1), Err(index2)) => with_span_interner(|interner| {
                interner.spans[index1].ctx == interner.spans[index2].ctx
            }),
        }
    }

    #[inline]
    pub fn with_parent(self, parent: Option<LocalDefId>) -> Span {
        let data = match_span_kind! {
            self,
            InlineCtx(span) => {
                // This format occurs 1-2 orders of magnitude more often than others (#126544),
                // so it makes sense to micro-optimize it to avoid `span.data()` and `Span::new()`.
                // Copypaste from `Span::new`, the small len & ctx conditions are known to hold.
                match parent {
                    None => return self,
                    Some(parent) => {
                        let parent32 = parent.local_def_index.as_u32();
                        if span.ctx == 0 && parent32 <= MAX_CTX {
                            return InlineParent::span(span.lo, span.len, parent32 as u16);
                        }
                    }
                }
                span.data()
            },
            InlineParent(span) => span.data(),
            PartiallyInterned(span) => span.data(),
            Interned(span) => span.data(),
        };

        if let Some(old_parent) = data.parent {
            (*SPAN_TRACK)(old_parent);
        }
        data.with_parent(parent)
    }

    #[inline]
    pub fn parent(self) -> Option<LocalDefId> {
        let interned_parent =
            |index: u32| with_span_interner(|interner| interner.spans[index as usize].parent);
        match_span_kind! {
            self,
            InlineCtx(_span) => None,
            InlineParent(span) => Some(LocalDefId { local_def_index: DefIndex::from_u16(span.parent) }),
            PartiallyInterned(span) => interned_parent(span.index),
            Interned(span) => interned_parent(span.index),
        }
    }
}

#[derive(Default)]
pub struct SpanInterner {
    spans: FxIndexSet<SpanData>,
}

impl SpanInterner {
    fn interner(&mut self, span_data: &SpanData) -> u32 {
        let (index, _) = self.spans.insert_full(*span_data);
        index as u32
    }
}

#[inline]
fn with_span_interner<T, F: FnOnce(&mut SpanInterner) -> T>(f: F) -> T {
    crate::with_session_globals(|session_globals| f(&mut session_globals.span_interner.lock()))
}
