use std::borrow::Cow;
use std::sync::Arc;
use std::{cmp, fmt};

pub use terebinth_ast_ir::{Movability, Mutability, Pinnedness};
use terebinth_data_structures::{
    packed::Pu128,
    stable_hasher::{HashStable, StableHasher},
    stack::ensure_sufficient_stack,
    tagged_ptr::Tag,
};
use terebinth_macros::{Decodable, Encodable, HashStable_Generic};
pub use terebinth_span::AttrId;
use terebinth_span::{
    ErrorGuaranteed, Ident, Span, Symbol, kw,
    source_map::{Spanned, respan},
    sym,
};
use thin_vec::{ThinVec, thin_vec};

pub use crate::format::*;
use crate::ptr::P;
use crate::token::{self, CommentKind, Delimiter};
use crate::tokenstream::{DelimSpan, LazyAttrTokenStream, TokenStream};
use crate::util::parser::{ExprPrecedence, Fixity};

/// A "Label" is an identifier of some point in sources,
/// e.g. in the following code:
///
/// ```terebinth
/// 'outer: loop {
///     break 'outer;
/// }
/// ```
///
/// `'outer` is a label.
#[derive(Clone, Encodable, Decodable, Copy, HashStable_Generic, Eq, PartialEq)]
pub struct Label {
    pub ident: Ident,
}

impl fmt::Debug for Label {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "label({:?})", self.ident)
    }
}

/// A "Path" is essentially Terebinth's notion of a name.
///
/// It's represented as a sequence of identifiers,
/// along with a bunch of supporting information.
///
/// E.g., `std::cmp::PartialEq`.
#[derive(Clone, Encodable, Decodable, Debug)]
pub struct Path {
    pub span: Span,
    /// The segments in the path: the things separated by `::`.
    /// Global paths begin with `kw::PathRoot`.
    pub segments: ThinVec<PathSegment>,
    pub tokens: Option<LazyAttrTokenStream>,
}

impl PartialEq<Symbol> for Path {
    #[inline]
    fn eq(&self, symbol: &Symbol) -> bool {
        matches!(&self.segments[..], [segment] if segment.ident.name == *symbol)
    }
}

impl<CTX: terebinth_span::HashStableContext> HashStable<CTX> for Path {
    fn hash_stable(&self, hcx: &mut CTX, hasher: &mut StableHasher) {
        self.segments.len().hash_stable(hcx, hasher);
        for segment in &self.segments {
            segment.ident.hash_stable(hcx, hasher);
        }
    }
}

impl Path {
    /// Convert a span and an identifier to the corresponding
    /// one-segment path.
    pub fn from_ident(ident: Ident) -> Path {
        Path {
            segments: thin_vec![PathSegment::from_ident(ident)],
            span: ident.span,
            tokens: None,
        }
    }

    pub fn is_global(&self) -> bool {
        self.segments
            .first()
            .is_some_and(|segment| segment.ident.name == kw::PathRoot)
    }

    /// Check if this path is potentially a trivial const arg, i.e., one that can _potentially_
    /// be represented without an anon const in the HIR.
    ///
    /// If `allow_mgca_arg` is true (as should be the case in most situations when
    /// `#![feature(min_generic_const_args)]` is enabled), then this always returns true
    /// because all paths are valid.
    ///
    /// Otherwise, it returns true iff the path has exactly one segment, and it has no generic args
    /// (i.e., it is _potentially_ a const parameter).
    #[tracing::instrument(level = "debug", ret)]
    pub fn is_potential_trivial_const_arg(&self, allow_mgca_arg: bool) -> bool {
        allow_mgca_arg
            || self.segments.len() == 1 && self.segments.iter().all(|seg| seg.args.is_none())
    }
}

/// A segment of a path: an identifier and a set of types.
///
/// E.g., `std`, `String` or `Box<T>`.
#[derive(Clone, Encodable, Decodable, Debug)]
pub struct PathSegment {
    /// The identifier portion of this path segment.
    pub ident: Ident,

    pub id: NodeId,

    /// Type/lifetime parameters attached to this path. They come in
    /// two flavors: `Path<A,B,C>` and `Path(A,B) -> C`.
    /// `None` means that no parameter list is supplied (`Path`),
    /// `Some` means that parameter list is supplied (`Path<X, Y>`)
    /// but it can be empty (`Path<>`).
    /// `P` is used as a size optimization for the common case with no parameters.
    pub args: Option<P<GenericArgs>>,
}

impl PathSegment {
    pub fn from_ident(ident: Ident) -> Self {
        PathSegment {
            ident,
            id: DUMMY_NODE_ID,
            args: None,
        }
    }

    pub fn path_root(span: Span) -> Self {
        PathSegment::from_ident(Ident::new(kw::PathRoot, span))
    }

    pub fn span(&self) -> Span {
        match &self.args {
            Some(args) => self.ident.span.to(args.span()),
            None => self.ident.span,
        }
    }
}

use crate::AstDeref;
pub use crate::node_id::{CRATE_NODE_ID, DUMMY_NODE_ID, NodeId};

/// A block (`{ .. }`).
///
/// E.g., `{ .. }` as in `fn foo() { .. }`.
#[derive(Clone, Encodable, Decodable, Debug)]
pub struct Block {
    /// The statements in the block.
    pub stmts: ThinVec<Stmt>,
    pub id: NodeId,
    /// Distinguishes between `unsafe { ... }` and `{ ... }`.
    pub rules: BlockCheckMode,
    pub span: Span,
    pub tokens: Option<LazyAttrTokenStream>,
    /// The following *isn't* a parse error, but will cause multiple errors in following stages.
    /// ```compile_fail
    /// let x = {
    ///     foo: var
    /// };
    /// ```
    /// #34255
    pub could_be_bare_literal: bool,
}

/// A match pattern.
///
/// Patterns appear in match statements and some other contexts, such as `let` and `if let`.
#[derive(Clone, Encodable, Decodable, Debug)]
pub struct Pattern {
    pub id: NodeId,
    pub kind: PatternKind,
    pub span: Span,
    pub tokens: Option<LazyAttrTokenStream>,
}

impl Pattern {
    /// Attempt reparsing the pattern as a type.
    /// This is intended for use by diagnostics.
    pub fn to_ty(&self) -> Option<P<Ty>> {
        let kind = match &self.kind {
            // In a type expression `_` is an inference variable.
            PatternKind::Wild => TyKind::Infer,
            // An IDENT pattern with no binding mode would be valid as path to a type. E.g. `u32`.
            PatternKind::Ident(BindingMode::NONE, ident, None) => {
                TyKind::Path(None, Path::from_ident(*ident))
            }
            PatternKind::Path(qself, path) => TyKind::Path(qself.clone(), path.clone()),
            // `&mut? P` can be reinterpreted as `&mut? T` where `T` is `P` reparsed as a type.
            PatternKind::Ref(pat, mutbl) => pat
                .to_ty()
                .map(|ty| TyKind::Ref(None, MutTy { ty, mutbl: *mutbl }))?,
            // A slice/array pattern `[P]` can be reparsed as `[T]`, an unsized array,
            // when `P` can be reparsed as a type `T`.
            PatternKind::Slice(pats) if let [pat] = pats.as_slice() => {
                pat.to_ty().map(TyKind::Slice)?
            }
            // A tuple pattern `(P0, .., Pn)` can be reparsed as `(T0, .., Tn)`
            // assuming `T0` to `Tn` are all syntactically valid as types.
            PatternKind::Tuple(pats) => {
                let mut tys = ThinVec::with_capacity(pats.len());
                // FIXME(#48994) - could just be collected into an Option<Vec>
                for pat in pats {
                    tys.push(pat.to_ty()?);
                }
                TyKind::Tup(tys)
            }
            _ => return None,
        };

        Some(P(Ty {
            kind,
            id: self.id,
            span: self.span,
            tokens: None,
        }))
    }

    /// Walk top-down and call `it` in each place where a pattern occurs
    /// starting with the root pattern `walk` is called on. If `it` returns
    /// false then we will descend no further but siblings will be processed.
    pub fn walk(&self, it: &mut impl FnMut(&Pattern) -> bool) {
        if !it(self) {
            return;
        }

        match &self.kind {
            // Walk into the pattern associated with `Ident` (if any).
            PatternKind::Ident(_, _, Some(p)) => p.walk(it),

            // Walk into each field of struct.
            PatternKind::Struct(_, _, fields, _) => {
                fields.iter().for_each(|field| field.pattern.walk(it))
            }

            // Sequence of patterns.
            PatternKind::TupleStruct(_, _, s)
            | PatternKind::Tuple(s)
            | PatternKind::Slice(s)
            | PatternKind::Or(s) => s.iter().for_each(|p| p.walk(it)),

            // Trivial wrappers over inner patterns.
            PatternKind::Box(s)
            | PatternKind::Deref(s)
            | PatternKind::Ref(s, _)
            | PatternKind::Paren(s)
            | PatternKind::Guard(s, _) => s.walk(it),

            // These patterns do not contain subpatterns, skip.
            PatternKind::Wild
            | PatternKind::Rest
            | PatternKind::Never
            | PatternKind::Expr(_)
            | PatternKind::Range(..)
            | PatternKind::Ident(..)
            | PatternKind::Path(..)
            | PatternKind::Err(_) => {}
        }
    }

    /// Is this a `..` pattern?
    pub fn is_rest(&self) -> bool {
        matches!(self.kind, PatternKind::Rest)
    }

    /// Whether this could be a never pattern, taking into account that a macro invocation can
    /// return a never pattern. Used to inform errors during parsing.
    pub fn could_be_never_pattern(&self) -> bool {
        let mut could_be_never_pattern = false;
        self.walk(&mut |pat| match &pat.kind {
            PatternKind::Never | PatternKind::MacCall(_) => {
                could_be_never_pattern = true;
                false
            }
            PatternKind::Or(s) => {
                could_be_never_pattern = s.iter().all(|p| p.could_be_never_pattern());
                false
            }
            _ => true,
        });
        could_be_never_pattern
    }

    /// Whether this contains a `!` pattern. This in particular means that a feature gate error will
    /// be raised if the feature is off. Used to avoid gating the feature twice.
    pub fn contains_never_pattern(&self) -> bool {
        let mut contains_never_pattern = false;
        self.walk(&mut |pat| {
            if matches!(pat.kind, PatternKind::Never) {
                contains_never_pattern = true;
            }
            true
        });
        contains_never_pattern
    }

    /// Return a name suitable for diagnostics.
    pub fn descr(&self) -> Option<String> {
        match &self.kind {
            PatternKind::Wild => Some("_".to_string()),
            PatternKind::Ident(BindingMode::NONE, ident, None) => Some(format!("{ident}")),
            PatternKind::Ref(pat, mutbl) => {
                pat.descr().map(|d| format!("&{}{d}", mutbl.prefix_str()))
            }
            _ => None,
        }
    }
}

/// A single field in a struct pattern.
///
/// Patterns like the fields of `Foo { x, ref y, ref mut z }`
/// are treated the same as `x: x, y: ref y, z: ref mut z`,
/// except when `is_shorthand` is true.
#[derive(Clone, Encodable, Decodable, Debug)]
pub struct PatternField {
    /// The identifier for the field.
    pub ident: Ident,
    /// The pattern the field is destructured to.
    pub pattern: P<Pattern>,
    pub is_shorthand: bool,
    pub id: NodeId,
    pub span: Span,
    pub is_placeholder: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Encodable, Decodable, HashStable_Generic)]
pub enum ByRef {
    Yes(Mutability),
    No,
}

impl ByRef {
    #[must_use]
    pub fn cap_ref_mutability(mut self, mutbl: Mutability) -> Self {
        if let ByRef::Yes(old_mutbl) = &mut self {
            *old_mutbl = cmp::min(*old_mutbl, mutbl);
        }
        self
    }
}

/// The mode of a binding (`mut`, `ref mut`, etc).
/// Used for both the explicit binding annotations given in the HIR for a binding
/// and the final binding mode that we infer after type inference/match ergonomics.
/// `.0` is the by-reference mode (`ref`, `ref mut`, or by value),
/// `.1` is the mutability of the binding.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Encodable, Decodable, HashStable_Generic)]
pub struct BindingMode(pub ByRef, pub Mutability);

impl BindingMode {
    pub const NONE: Self = Self(ByRef::No, Mutability::Not);
    pub const REF: Self = Self(ByRef::Yes(Mutability::Not), Mutability::Not);
    pub const MUT: Self = Self(ByRef::No, Mutability::Mut);
    pub const REF_MUT: Self = Self(ByRef::Yes(Mutability::Mut), Mutability::Not);
    pub const MUT_REF: Self = Self(ByRef::Yes(Mutability::Not), Mutability::Mut);
    pub const MUT_REF_MUT: Self = Self(ByRef::Yes(Mutability::Mut), Mutability::Mut);

    pub fn prefix_str(self) -> &'static str {
        match self {
            Self::NONE => "",
            Self::REF => "ref ",
            Self::MUT => "mut ",
            Self::REF_MUT => "ref mut ",
            Self::MUT_REF => "mut ref ",
            Self::MUT_REF_MUT => "mut ref mut ",
        }
    }
}

#[derive(Clone, Encodable, Decodable, Debug)]
pub enum RangeEnd {
    /// `..=` or `...`
    Included(RangeSyntax),
    /// `..`
    Excluded,
}

#[derive(Clone, Encodable, Decodable, Debug)]
pub enum RangeSyntax {
    /// `...`
    DotDotDot,
    /// `..=`
    DotDotEq,
}

/// All the different flavors of pattern that Rust recognizes.
//
// Adding a new variant? Please update `test_pat` in `tests/ui/macros/stringify.rs`.
#[derive(Clone, Encodable, Decodable, Debug)]
pub enum PatternKind {
    /// Represents a wildcard pattern (`_`).
    Wild,

    /// A `PatKind::Ident` may either be a new bound variable (`ref mut binding @ OPT_SUBPATTERN`),
    /// or a unit struct/variant pattern, or a const pattern (in the last two cases the third
    /// field must be `None`). Disambiguation cannot be done with parser alone, so it happens
    /// during name resolution.
    Ident(BindingMode, Ident, Option<P<Pattern>>),

    /// A struct or struct variant pattern (e.g., `Variant {x, y, ..}`).
    Struct(Option<P<QSelf>>, Path, ThinVec<PatternField>, PatFieldsRest),

    /// A tuple struct/variant pattern (`Variant(x, y, .., z)`).
    TupleStruct(Option<P<QSelf>>, Path, ThinVec<P<Pattern>>),

    /// An or-pattern `A | B | C`.
    /// Invariant: `pats.len() >= 2`.
    Or(ThinVec<P<Pattern>>),

    /// A possibly qualified path pattern.
    /// Unqualified path patterns `A::B::C` can legally refer to variants, structs, constants
    /// or associated constants. Qualified path patterns `<A>::B::C`/`<A as Trait>::B::C` can
    /// only legally refer to associated constants.
    Path(Option<P<QSelf>>, Path),

    /// A tuple pattern (`(a, b)`).
    Tuple(ThinVec<P<Pattern>>),

    /// A `box` pattern.
    Box(P<Pattern>),

    /// A `deref` pattern (currently `deref!()` macro-based syntax).
    Deref(P<Pattern>),

    /// A reference pattern (e.g., `&mut (a, b)`).
    Ref(P<Pattern>, Mutability),

    /// A literal, const block or path.
    Expr(P<Expr>),

    /// A range pattern (e.g., `1...2`, `1..2`, `1..`, `..2`, `1..=2`, `..=2`).
    Range(Option<P<Expr>>, Option<P<Expr>>, Spanned<RangeEnd>),

    /// A slice pattern `[a, b, c]`.
    Slice(ThinVec<P<Pattern>>),

    /// A rest pattern `..`.
    ///
    /// Syntactically it is valid anywhere.
    ///
    /// Semantically however, it only has meaning immediately inside:
    /// - a slice pattern: `[a, .., b]`,
    /// - a binding pattern immediately inside a slice pattern: `[a, r @ ..]`,
    /// - a tuple pattern: `(a, .., b)`,
    /// - a tuple struct/variant pattern: `$path(a, .., b)`.
    ///
    /// In all of these cases, an additional restriction applies,
    /// only one rest pattern may occur in the pattern sequences.
    Rest,

    // A never pattern `!`.
    Never,

    /// A guard pattern (e.g., `x if guard(x)`).
    Guard(P<Pattern>, P<Expr>),

    /// Parentheses in patterns used for grouping (i.e., `(PAT)`).
    Paren(P<Pattern>),

    /// Placeholder for a pattern that wasn't syntactically well formed in some way.
    Err(ErrorGuaranteed),
}

/// Whether the `..` is present in a struct fields pattern.
#[derive(Clone, Copy, Encodable, Decodable, Debug, PartialEq)]
pub enum PatFieldsRest {
    /// `module::StructName { field, ..}`
    Rest,
    /// `module::StructName { field, syntax error }`
    Recovered(ErrorGuaranteed),
    /// `module::StructName { field }`
    None,
}

/// The kind of borrow in an `AddrOf` expression,
/// e.g., `&place` or `&raw const place`.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Encodable, Decodable, HashStable_Generic)]
pub enum BorrowKind {
    /// A normal borrow, `&$expr` or `&mut $expr`.
    /// The resulting type is either `&'a T` or `&'a mut T`
    /// where `T = typeof($expr)` and `'a` is some lifetime.
    Ref,
    /// A raw borrow, `&raw const $expr` or `&raw mut $expr`.
    /// The resulting type is either `*const T` or `*mut T`
    /// where `T = typeof($expr)`.
    Raw,
}

#[derive(Clone, Copy, Debug, PartialEq, Encodable, Decodable, HashStable_Generic)]
pub enum BinaryOperatorKind {
    /// The `+` operator (addition)
    Add,
    /// The `-` operator (subtraction)
    Sub,
    /// The `*` operator (multiplication)
    Mul,
    /// The `/` operator (division)
    Div,
    /// The `%` operator (modulus)
    Rem,
    /// The `&&` operator (logical and)
    And,
    /// The `||` operator (logical or)
    Or,
    /// The `^` operator (bitwise xor)
    BitXor,
    /// The `&` operator (bitwise and)
    BitAnd,
    /// The `|` operator (bitwise or)
    BitOr,
    /// The `<<` operator (shift left)
    Shl,
    /// The `>>` operator (shift right)
    Shr,
    /// The `==` operator (equality)
    Eq,
    /// The `<` operator (less than)
    Lt,
    /// The `<=` operator (less than or equal to)
    Le,
    /// The `!=` operator (not equal to)
    Ne,
    /// The `>=` operator (greater than or equal to)
    Ge,
    /// The `>` operator (greater than)
    Gt,
}

impl BinaryOperatorKind {
    pub fn as_str(&self) -> &'static str {
        use BinaryOperatorKind::*;
        match self {
            Add => "+",
            Sub => "-",
            Mul => "*",
            Div => "/",
            Rem => "%",
            And => "&&",
            Or => "||",
            BitXor => "^",
            BitAnd => "&",
            BitOr => "|",
            Shl => "<<",
            Shr => ">>",
            Eq => "==",
            Lt => "<",
            Le => "<=",
            Ne => "!=",
            Ge => ">=",
            Gt => ">",
        }
    }

    pub fn is_lazy(&self) -> bool {
        matches!(self, BinaryOperatorKind::And | BinaryOperatorKind::Or)
    }

    pub fn precedence(&self) -> ExprPrecedence {
        use BinaryOperatorKind::*;
        match *self {
            Mul | Div | Rem => ExprPrecedence::Product,
            Add | Sub => ExprPrecedence::Sum,
            Shl | Shr => ExprPrecedence::Shift,
            BitAnd => ExprPrecedence::BitAnd,
            BitXor => ExprPrecedence::BitXor,
            BitOr => ExprPrecedence::BitOr,
            Lt | Gt | Le | Ge | Eq | Ne => ExprPrecedence::Compare,
            And => ExprPrecedence::LAnd,
            Or => ExprPrecedence::LOr,
        }
    }

    pub fn fixity(&self) -> Fixity {
        use BinaryOperatorKind::*;
        match self {
            Eq | Ne | Lt | Le | Gt | Ge => Fixity::None,
            Add | Sub | Mul | Div | Rem | And | Or | BitXor | BitAnd | BitOr | Shl | Shr => {
                Fixity::Left
            }
        }
    }

    pub fn is_comparison(self) -> bool {
        use BinaryOperatorKind::*;
        match self {
            Eq | Ne | Lt | Le | Gt | Ge => true,
            Add | Sub | Mul | Div | Rem | And | Or | BitXor | BitAnd | BitOr | Shl | Shr => false,
        }
    }

    /// Returns `true` if the binary operator takes its arguments by value.
    pub fn is_by_value(self) -> bool {
        !self.is_comparison()
    }
}

pub type BinOp = Spanned<BinaryOperatorKind>;

/// Unary operator.
///
/// Note that `&data` is not an operator, it's an `AddrOf` expression.
#[derive(Clone, Copy, Debug, PartialEq, Encodable, Decodable, HashStable_Generic)]
pub enum UnaryOperator {
    /// The `*` operator for dereferencing
    Deref,
    /// The `!` operator for logical inversion
    Not,
    /// The `-` operator for negation
    Neg,
}

impl UnaryOperator {
    pub fn as_str(&self) -> &'static str {
        match self {
            UnaryOperator::Deref => "*",
            UnaryOperator::Not => "!",
            UnaryOperator::Neg => "-",
        }
    }

    /// Returns `true` if the unary operator takes its argument by value.
    pub fn is_by_value(self) -> bool {
        matches!(self, Self::Neg | Self::Not)
    }
}

/// A statement. No `attrs` or `tokens` fields because each `StmtKind` variant
/// contains an AST node with those fields. (Except for `StmtKind::Empty`,
/// which never has attrs or tokens)
#[derive(Clone, Encodable, Decodable, Debug)]
pub struct Stmt {
    pub id: NodeId,
    pub kind: StmtKind,
    pub span: Span,
}

impl Stmt {
    pub fn has_trailing_semicolon(&self) -> bool {
        match &self.kind {
            StmtKind::Semi(_) => true,
            StmtKind::MacCall(mac) => matches!(mac.style, MacStmtStyle::Semicolon),
            _ => false,
        }
    }

    /// Converts a parsed `Stmt` to a `Stmt` with
    /// a trailing semicolon.
    ///
    /// This only modifies the parsed AST struct, not the attached
    /// `LazyAttrTokenStream`. The parser is responsible for calling
    /// `ToAttrTokenStream::add_trailing_semi` when there is actually
    /// a semicolon in the tokenstream.
    pub fn add_trailing_semicolon(mut self) -> Self {
        self.kind = match self.kind {
            StmtKind::Expr(expr) => StmtKind::Semi(expr),
            StmtKind::MacCall(mac) => StmtKind::MacCall(mac.map(
                |MacCallStmt {
                     mac,
                     style: _,
                     attrs,
                     tokens,
                 }| {
                    MacCallStmt {
                        mac,
                        style: MacStmtStyle::Semicolon,
                        attrs,
                        tokens,
                    }
                },
            )),
            kind => kind,
        };

        self
    }

    pub fn is_item(&self) -> bool {
        matches!(self.kind, StmtKind::Item(_))
    }

    pub fn is_expr(&self) -> bool {
        matches!(self.kind, StmtKind::Expr(_))
    }
}

// Adding a new variant? Please update `test_stmt` in `tests/ui/macros/stringify.rs`.
#[derive(Clone, Encodable, Decodable, Debug)]
pub enum StmtKind {
    /// A local (let) binding.
    Let(P<Local>),
    /// An item definition.
    Item(P<Item>),
    /// Expr without trailing semi-colon.
    Expr(P<Expr>),
    /// Expr with a trailing semi-colon.
    Semi(P<Expr>),
    /// Just a trailing semi-colon.
    Empty,
}

/// Local represents a `let` statement, e.g., `let <pat>:<ty> = <expr>;`.
#[derive(Clone, Encodable, Decodable, Debug)]
pub struct Local {
    pub id: NodeId,
    pub pat: P<Pattern>,
    pub ty: Option<P<Ty>>,
    pub kind: LocalKind,
    pub span: Span,
    pub colon_sp: Option<Span>,
    pub tokens: Option<LazyAttrTokenStream>,
}

#[derive(Clone, Encodable, Decodable, Debug)]
pub enum LocalKind {
    /// Local declaration.
    /// Example: `let x;`
    Decl,
    /// Local declaration with an initializer.
    /// Example: `let x = y;`
    Init(P<Expr>),
    /// Local declaration with an initializer and an `else` clause.
    /// Example: `let Some(x) = y else { return };`
    InitElse(P<Expr>, P<Block>),
}

impl LocalKind {
    pub fn init(&self) -> Option<&Expr> {
        match self {
            Self::Decl => None,
            Self::Init(i) | Self::InitElse(i, _) => Some(i),
        }
    }

    pub fn init_else_opt(&self) -> Option<(&Expr, Option<&Block>)> {
        match self {
            Self::Decl => None,
            Self::Init(init) => Some((init, None)),
            Self::InitElse(init, els) => Some((init, Some(els))),
        }
    }
}

/// An arm of a 'match'.
///
/// E.g., `0..=10 => { println!("match!") }` as in
///
/// ```
/// match 123 {
///     0..=10 => { println!("match!") },
///     _ => { println!("no match!") },
/// }
/// ```
#[derive(Clone, Encodable, Decodable, Debug)]
pub struct Arm {
    pub attrs: AttrVec,
    /// Match arm pattern, e.g. `10` in `match foo { 10 => {}, _ => {} }`.
    pub pat: P<Pattern>,
    /// Match arm guard, e.g. `n > 10` in `match foo { n if n > 10 => {}, _ => {} }`.
    pub guard: Option<P<Expr>>,
    /// Match arm body. Omitted if the pattern is a never pattern.
    pub body: Option<P<Expr>>,
    pub span: Span,
    pub id: NodeId,
    pub is_placeholder: bool,
}

/// A single field in a struct expression, e.g. `x: value` and `y` in `Foo { x: value, y }`.
#[derive(Clone, Encodable, Decodable, Debug)]
pub struct ExprField {
    pub attrs: AttrVec,
    pub id: NodeId,
    pub span: Span,
    pub ident: Ident,
    pub expr: P<Expr>,
    pub is_shorthand: bool,
    pub is_placeholder: bool,
}

#[derive(Clone, PartialEq, Encodable, Decodable, Debug, Copy)]
pub enum BlockCheckMode {
    Default,
    Unsafe(UnsafeSource),
}

#[derive(Clone, PartialEq, Encodable, Decodable, Debug, Copy)]
pub enum UnsafeSource {
    CompilerGenerated,
    UserProvided,
}

/// A constant (expression) that's not an item or associated item,
/// but needs its own `DefId` for type-checking, const-eval, etc.
/// These are usually found nested inside types (e.g., array lengths)
/// or expressions (e.g., repeat counts), and also used to define
/// explicit discriminant values for enum variants.
#[derive(Clone, Encodable, Decodable, Debug)]
pub struct AnonConst {
    pub id: NodeId,
    pub value: P<Expr>,
}

/// An expression.
#[derive(Clone, Encodable, Decodable, Debug)]
pub struct Expr {
    pub id: NodeId,
    pub kind: ExprKind,
    pub span: Span,
    pub attrs: AttrVec,
    pub tokens: Option<LazyAttrTokenStream>,
}

impl Expr {
    /// Check if this expression is potentially a trivial const arg, i.e., one that can _potentially_
    /// be represented without an anon const in the HIR.
    ///
    /// This will unwrap at most one block level (curly braces). After that, if the expression
    /// is a path, it mostly dispatches to [`Path::is_potential_trivial_const_arg`].
    /// See there for more info about `allow_mgca_arg`.
    ///
    /// The only additional thing to note is that when `allow_mgca_arg` is false, this function
    /// will only allow paths with no qself, before dispatching to the `Path` function of
    /// the same name.
    ///
    /// Does not ensure that the path resolves to a const param/item, the caller should check this.
    /// This also does not consider macros, so it's only correct after macro-expansion.
    pub fn is_potential_trivial_const_arg(&self, allow_mgca_arg: bool) -> bool {
        let this = self.maybe_unwrap_block();
        if allow_mgca_arg {
            matches!(this.kind, ExprKind::Path(..))
        } else {
            if let ExprKind::Path(None, path) = &this.kind
                && path.is_potential_trivial_const_arg(allow_mgca_arg)
            {
                true
            } else {
                false
            }
        }
    }

    /// Returns an expression with (when possible) *one* outter brace removed
    pub fn maybe_unwrap_block(&self) -> &Expr {
        if let ExprKind::Block(block, None) = &self.kind
            && let [stmt] = block.stmts.as_slice()
            && let StmtKind::Expr(expr) = &stmt.kind
        {
            expr
        } else {
            self
        }
    }

    pub fn peel_parens(&self) -> &Expr {
        let mut expr = self;
        while let ExprKind::Paren(inner) = &expr.kind {
            expr = inner;
        }
        expr
    }

    pub fn peel_parens_and_refs(&self) -> &Expr {
        let mut expr = self;
        while let ExprKind::Paren(inner) | ExprKind::AddrOf(BorrowKind::Ref, _, inner) = &expr.kind
        {
            expr = inner;
        }
        expr
    }

    /// Attempts to reparse as `Ty` (for diagnostic purposes).
    pub fn to_ty(&self) -> Option<P<Ty>> {
        let kind = match &self.kind {
            // Trivial conversions.
            ExprKind::Path(qself, path) => TyKind::Path(qself.clone(), path.clone()),

            ExprKind::Paren(expr) => expr.to_ty().map(TyKind::Paren)?,

            ExprKind::AddrOf(BorrowKind::Ref, mutbl, expr) => expr
                .to_ty()
                .map(|ty| TyKind::Ref(None, MutTy { ty, mutbl: *mutbl }))?,

            ExprKind::Repeat(expr, expr_len) => {
                expr.to_ty().map(|ty| TyKind::Array(ty, expr_len.clone()))?
            }

            ExprKind::Array(exprs) if let [expr] = exprs.as_slice() => {
                expr.to_ty().map(TyKind::Slice)?
            }

            ExprKind::Tup(exprs) => {
                let tys = exprs
                    .iter()
                    .map(|expr| expr.to_ty())
                    .collect::<Option<ThinVec<_>>>()?;
                TyKind::Tup(tys)
            }

            // If binary operator is `Add` and both `lhs` and `rhs` are trait bounds,
            // then type of result is trait object.
            // Otherwise we don't assume the result type.
            ExprKind::Binary(binop, lhs, rhs) if binop.node == BinaryOperatorKind::Add => {
                if let (Some(lhs), Some(rhs)) = (lhs.to_bound(), rhs.to_bound()) {
                    TyKind::TraitObject(vec![lhs, rhs], TraitObjectSyntax::None)
                } else {
                    return None;
                }
            }

            ExprKind::Underscore => TyKind::Infer,

            // This expression doesn't look like a type syntactically.
            _ => return None,
        };

        Some(P(Ty {
            kind,
            id: self.id,
            span: self.span,
            tokens: None,
        }))
    }

    pub fn precedence(&self) -> ExprPrecedence {
        match &self.kind {
            ExprKind::Closure(closure) => {
                match closure.fn_decl.output {
                    FnRetTy::Default(_) => ExprPrecedence::Jump,
                    FnRetTy::Ty(_) => ExprPrecedence::Unambiguous,
                }
            }

            ExprKind::Break(..)
            | ExprKind::Ret(..)
            | ExprKind::Yield(..)
            | ExprKind::Become(..) => ExprPrecedence::Jump,

            // `Range` claims to have higher precedence than `Assign`, but `x .. x = x` fails to
            // parse, instead of parsing as `(x .. x) = x`. Giving `Range` a lower precedence
            // ensures that `pprust` will add parentheses in the right places to get the desired
            // parse.
            ExprKind::Range(..) => ExprPrecedence::Range,

            // Binop-like expr kinds, handled by `AssocOp`.
            ExprKind::Binary(op, ..) => op.node.precedence(),
            ExprKind::Cast(..) => ExprPrecedence::Cast,

            ExprKind::Assign(..) |
            ExprKind::AssignOp(..) => ExprPrecedence::Assign,

            // Unary, prefix
            ExprKind::AddrOf(..)
            // Here `let pats = expr` has `let pats =` as a "unary" prefix of `expr`.
            // However, this is not exactly right. When `let _ = a` is the LHS of a binop we
            // need parens sometimes. E.g. we can print `(let _ = a) && b` as `let _ = a && b`
            // but we need to print `(let _ = a) < b` as-is with parens.
            | ExprKind::Let(..)
            | ExprKind::Unary(..) => ExprPrecedence::Prefix,

            // Never need parens
            ExprKind::Array(_)
            | ExprKind::Await(..)
            | ExprKind::Use(..)
            | ExprKind::Block(..)
            | ExprKind::Call(..)
            | ExprKind::ConstBlock(_)
            | ExprKind::Continue(..)
            | ExprKind::Field(..)
            | ExprKind::ForLoop { .. }
            | ExprKind::FormatArgs(..)
            | ExprKind::If(..)
            | ExprKind::IncludedBytes(..)
            | ExprKind::Index(..)
            | ExprKind::Lit(_)
            | ExprKind::Loop(..)
            | ExprKind::Match(..)
            | ExprKind::MethodCall(..)
            | ExprKind::OffsetOf(..)
            | ExprKind::Paren(..)
            | ExprKind::Path(..)
            | ExprKind::Repeat(..)
            | ExprKind::Struct(..)
            | ExprKind::Try(..)
            | ExprKind::TryBlock(..)
            | ExprKind::Tup(_)
            | ExprKind::Type(..)
            | ExprKind::Underscore
            | ExprKind::While(..)
            | ExprKind::Err(_)
            | ExprKind::Dummy => ExprPrecedence::Unambiguous,
        }
    }

    /// To a first-order approximation, is this a pattern?
    pub fn is_approximately_pattern(&self) -> bool {
        matches!(
            &self.peel_parens().kind,
            ExprKind::Array(_)
                | ExprKind::Call(_, _)
                | ExprKind::Tup(_)
                | ExprKind::Lit(_)
                | ExprKind::Range(_, _, _)
                | ExprKind::Underscore
                | ExprKind::Path(_, _)
                | ExprKind::Struct(_)
        )
    }
}

#[derive(Clone, Encodable, Decodable, Debug)]
pub struct Closure {
    pub binder: ClosureBinder,
    pub capture_clause: CaptureBy,
    pub constness: Const,
    pub coroutine_kind: Option<CoroutineKind>,
    pub movability: Movability,
    pub fn_decl: P<FnDecl>,
    pub body: P<Expr>,
    /// The span of the declaration block: 'move |...| -> ...'
    pub fn_decl_span: Span,
    /// The span of the argument block `|...|`
    pub fn_arg_span: Span,
}

/// Limit types of a range (inclusive or exclusive).
#[derive(Copy, Clone, PartialEq, Encodable, Decodable, Debug)]
pub enum RangeLimits {
    /// Inclusive at the beginning, exclusive at the end.
    HalfOpen,
    /// Inclusive at the beginning and end.
    Closed,
}

impl RangeLimits {
    pub fn as_str(&self) -> &'static str {
        match self {
            RangeLimits::HalfOpen => "..",
            RangeLimits::Closed => "..=",
        }
    }
}

/// A method call (e.g. `x.foo::<Bar, Baz>(a, b, c)`).
#[derive(Clone, Encodable, Decodable, Debug)]
pub struct MethodCall {
    /// The method name and its generic arguments, e.g. `foo::<Bar, Baz>`.
    pub seg: PathSegment,
    /// The receiver, e.g. `x`.
    pub receiver: P<Expr>,
    /// The arguments, e.g. `a, b, c`.
    pub args: ThinVec<P<Expr>>,
    /// The span of the function, without the dot and receiver e.g. `foo::<Bar,
    /// Baz>(a, b, c)`.
    pub span: Span,
}

#[derive(Clone, Encodable, Decodable, Debug)]
pub enum StructRest {
    /// `..x`.
    Base(P<Expr>),
    /// `..`.
    Rest(Span),
    /// No trailing `..` or expression.
    None,
}

#[derive(Clone, Encodable, Decodable, Debug)]
pub struct StructExpr {
    pub qself: Option<P<QSelf>>,
    pub path: Path,
    pub fields: ThinVec<ExprField>,
    pub rest: StructRest,
}

// Adding a new variant? Please update `test_expr` in `tests/ui/macros/stringify.rs`.
#[derive(Clone, Encodable, Decodable, Debug)]
pub enum ExprKind {
    /// An array (e.g, `[a, b, c, d]`).
    Array(ThinVec<P<Expr>>),
    /// Allow anonymous constants from an inline `const` block.
    ConstBlock(AnonConst),
    /// A function call.
    ///
    /// The first field resolves to the function itself,
    /// and the second field is the list of arguments.
    /// This also represents calling the constructor of
    /// tuple-like ADTs such as tuple structs and enum variants.
    Call(P<Expr>, ThinVec<P<Expr>>),
    /// A method call (e.g., `x.foo::<Bar, Baz>(a, b, c)`).
    MethodCall(Box<MethodCall>),
    /// A tuple (e.g., `(a, b, c, d)`).
    Tup(ThinVec<P<Expr>>),
    /// A binary operation (e.g., `a + b`, `a * b`).
    Binary(BinOp, P<Expr>, P<Expr>),
    /// A unary operation (e.g., `!x`, `*x`).
    Unary(UnaryOperator, P<Expr>),
    /// A literal (e.g., `1`, `"foo"`).
    Lit(token::Lit),
    /// A cast (e.g., `foo as f64`).
    Cast(P<Expr>, P<Ty>),
    /// A type ascription (e.g., `builtin # type_ascribe(42, usize)`).
    ///
    /// Usually not written directly in user code but
    /// indirectly via the macro `type_ascribe!(...)`.
    Type(P<Expr>, P<Ty>),
    /// A `let pat = expr` expression that is only semantically allowed in the condition
    /// of `if` / `while` expressions. (e.g., `if let 0 = x { .. }`).
    ///
    /// `Span` represents the whole `let pat = expr` statement.
    Let(P<Pattern>, P<Expr>, Span, Recovered),
    /// An `if` block, with an optional `else` block.
    ///
    /// `if expr { block } else { expr }`
    If(P<Expr>, P<Block>, Option<P<Expr>>),
    /// A while loop, with an optional label.
    ///
    /// `'label: while expr { block }`
    While(P<Expr>, P<Block>, Option<Label>),
    /// A `for` loop, with an optional label.
    ///
    /// `'label: for await? pat in iter { block }`
    ///
    /// This is desugared to a combination of `loop` and `match` expressions.
    ForLoop {
        pat: P<Pattern>,
        iter: P<Expr>,
        body: P<Block>,
        label: Option<Label>,
        kind: ForLoopKind,
    },
    /// Conditionless loop (can be exited with `break`, `continue`, or `return`).
    ///
    /// `'label: loop { block }`
    Loop(P<Block>, Option<Label>, Span),
    /// A `match` block.
    Match(P<Expr>, ThinVec<Arm>, MatchKind),
    /// A closure (e.g., `move |a, b, c| a + b + c`).
    Closure(Box<Closure>),
    /// A block (`'label: { ... }`).
    Block(P<Block>, Option<Label>),
    /// An await expression (`my_future.await`). Span is of await keyword.
    Await(P<Expr>, Span),
    /// A use expression (`x.use`). Span is of use keyword.
    Use(P<Expr>, Span),

    /// A try block (`try { ... }`).
    TryBlock(P<Block>),

    /// An assignment (`a = foo()`).
    /// The `Span` argument is the span of the `=` token.
    Assign(P<Expr>, P<Expr>, Span),
    /// An assignment with an operator.
    ///
    /// E.g., `a += 1`.
    AssignOp(BinOp, P<Expr>, P<Expr>),
    /// Access of a named (e.g., `obj.foo`) or unnamed (e.g., `obj.0`) struct field.
    Field(P<Expr>, Ident),
    /// An indexing operation (e.g., `foo[2]`).
    /// The span represents the span of the `[2]`, including brackets.
    Index(P<Expr>, P<Expr>, Span),
    /// A range (e.g., `1..2`, `1..`, `..2`, `1..=2`, `..=2`; and `..` in destructuring assignment).
    Range(Option<P<Expr>>, Option<P<Expr>>, RangeLimits),
    /// An underscore, used in destructuring assignment to ignore a value.
    Underscore,

    /// Variable reference, possibly containing `::` and/or type
    /// parameters (e.g., `foo::bar::<baz>`).
    ///
    /// Optionally "qualified" (e.g., `<Vec<T> as SomeTrait>::SomeType`).
    Path(Option<P<QSelf>>, Path),

    /// A referencing operation (`&a`, `&mut a`, `&raw const a` or `&raw mut a`).
    AddrOf(BorrowKind, Mutability, P<Expr>),
    /// A `break`, with an optional label to break, and an optional expression.
    Break(Option<Label>, Option<P<Expr>>),
    /// A `continue`, with an optional label.
    Continue(Option<Label>),
    /// A `return`, with an optional value to be returned.
    Ret(Option<P<Expr>>),

    /// An `offset_of` expression (e.g., `builtin # offset_of(Struct, field)`).
    ///
    /// Usually not written directly in user code but
    /// indirectly via the macro `core::mem::offset_of!(...)`.
    OffsetOf(P<Ty>, P<[Ident]>),

    /// A struct literal expression.
    ///
    /// E.g., `Foo {x: 1, y: 2}`, or `Foo {x: 1, .. rest}`.
    Struct(P<StructExpr>),

    /// An array literal constructed from one repeated element.
    ///
    /// E.g., `[1; 5]`. The expression is the element to be
    /// repeated; the constant is the number of times to repeat it.
    Repeat(P<Expr>, AnonConst),

    /// No-op: used solely so we can pretty-print faithfully.
    Paren(P<Expr>),

    /// A try expression (`expr?`).
    Try(P<Expr>),

    /// A `yield`, with an optional value to be yielded.
    Yield(Option<P<Expr>>),

    /// A tail call return, with the value to be returned.
    ///
    /// While `.0` must be a function call, we check this later, after parsing.
    Become(P<Expr>),

    /// Bytes included via `include_bytes!`
    /// Added for optimization purposes to avoid the need to escape
    /// large binary blobs - should always behave like [`ExprKind::Lit`]
    /// with a `ByteStr` literal.
    IncludedBytes(Arc<[u8]>),

    /// A `format_args!()` expression.
    FormatArgs(P<FormatArgs>),

    /// Placeholder for an expression that wasn't syntactically well formed in some way.
    Err(ErrorGuaranteed),

    /// Acts as a null expression. Lowering it will always emit a bug.
    Dummy,
}

/// Used to differentiate between `for` loops and `for await` loops.
#[derive(Clone, Copy, Encodable, Decodable, Debug, PartialEq, Eq)]
pub enum ForLoopKind {
    For,
    ForAwait,
}

/// The explicit `Self` type in a "qualified path". The actual
/// path, including the trait and the associated item, is stored
/// separately. `position` represents the index of the associated
/// item qualified with this `Self` type.
///
/// ```ignore (only-for-syntax-highlight)
/// <Vec<T> as a::b::Trait>::AssociatedItem
///  ^~~~~     ~~~~~~~~~~~~~~^
///  ty        position = 3
///
/// <Vec<T>>::AssociatedItem
///  ^~~~~    ^
///  ty       position = 0
/// ```
#[derive(Clone, Encodable, Decodable, Debug)]
pub struct QSelf {
    pub ty: P<Ty>,

    /// The span of `a::b::Trait` in a path like `<Vec<T> as
    /// a::b::Trait>::AssociatedItem`; in the case where `position ==
    /// 0`, this is an empty span.
    pub path_span: Span,
    pub position: usize,
}

/// A capture clause used in closures and `async` blocks.
#[derive(Clone, Copy, PartialEq, Encodable, Decodable, Debug, HashStable_Generic)]
pub enum CaptureBy {
    /// `move |x| y + x`.
    Value {
        /// The span of the `move` keyword.
        move_kw: Span,
    },
    /// `move` or `use` keywords were not specified.
    Ref,
    /// `use |x| y + x`.
    ///
    /// Note that if you have a regular closure like `|| x.use`, this will *not* result
    /// in a `Use` capture. Instead, the `ExprUseVisitor` will look at the type
    /// of `x` and treat `x.use` as either a copy/clone/move as appropriate.
    Use {
        /// The span of the `use` keyword.
        use_kw: Span,
    },
}

/// Closure lifetime binder, `for<'a, 'b>` in `for<'a, 'b> |_: &'a (), _: &'b ()|`.
#[derive(Clone, Encodable, Decodable, Debug)]
pub enum ClosureBinder {
    /// The binder is not present, all closure lifetimes are inferred.
    NotPresent,
    /// The binder is present.
    For {
        /// Span of the whole `for<>` clause
        ///
        /// ```text
        /// for<'a, 'b> |_: &'a (), _: &'b ()| { ... }
        /// ^^^^^^^^^^^ -- this
        /// ```
        span: Span,

        /// Lifetimes in the `for<>` closure
        ///
        /// ```text
        /// for<'a, 'b> |_: &'a (), _: &'b ()| { ... }
        ///     ^^^^^^ -- this
        /// ```
        generic_params: ThinVec<GenericParam>,
    },
}

#[derive(Clone, Encodable, Decodable, Debug, Copy, Hash, Eq, PartialEq, HashStable_Generic)]
pub enum StrStyle {
    /// A regular string, like `"foo"`.
    Cooked,
    /// A raw string, like `r##"foo"##`.
    ///
    /// The value is the number of `#` symbols used.
    Raw(u8),
}

/// The kind of match expression
#[derive(Clone, Copy, Encodable, Decodable, Debug, PartialEq)]
pub enum MatchKind {
    /// match expr { ... }
    Prefix,
    /// expr.match { ... }
    Postfix,
}

/// A literal in a meta item.
#[derive(Clone, Encodable, Decodable, Debug, HashStable_Generic)]
pub struct MetaItemLit {
    /// The original literal as written in the source code.
    pub symbol: Symbol,
    /// The original suffix as written in the source code.
    pub suffix: Option<Symbol>,
    /// The "semantic" representation of the literal lowered from the original tokens.
    /// Strings are unescaped, hexadecimal forms are eliminated, etc.
    pub kind: LitKind,
    pub span: Span,
}

/// Similar to `MetaItemLit`, but restricted to string literals.
#[derive(Clone, Copy, Encodable, Decodable, Debug)]
pub struct StrLit {
    /// The original literal as written in source code.
    pub symbol: Symbol,
    /// The original suffix as written in source code.
    pub suffix: Option<Symbol>,
    /// The semantic (unescaped) representation of the literal.
    pub symbol_unescaped: Symbol,
    pub style: StrStyle,
    pub span: Span,
}

impl StrLit {
    pub fn as_token_lit(&self) -> token::Lit {
        let token_kind = match self.style {
            StrStyle::Cooked => token::Str,
            StrStyle::Raw(n) => token::StrRaw(n),
        };
        token::Lit::new(token_kind, self.symbol, self.suffix)
    }
}

/// Type of the integer literal based on provided suffix.
#[derive(Clone, Copy, Encodable, Decodable, Debug, Hash, Eq, PartialEq, HashStable_Generic)]
pub enum LitIntType {
    /// e.g. `42_i32`.
    Signed(IntTy),
    /// e.g. `42_u32`.
    Unsigned(UintTy),
    /// e.g. `42`.
    Unsuffixed,
}

/// Type of the float literal based on provided suffix.
#[derive(Clone, Copy, Encodable, Decodable, Debug, Hash, Eq, PartialEq, HashStable_Generic)]
pub enum LitFloatType {
    /// A float literal with a suffix (`1f32` or `1E10f32`).
    Suffixed(FloatTy),
    /// A float literal without a suffix (`1.0 or 1.0E10`).
    Unsuffixed,
}

/// This type is used within both `ast::MetaItemLit` and `hir::Lit`.
///
/// Note that the entire literal (including the suffix) is considered when
/// deciding the `LitKind`. This means that float literals like `1f32` are
/// classified by this type as `Float`. This is different to `token::LitKind`
/// which does *not* consider the suffix.
#[derive(Clone, Encodable, Decodable, Debug, Hash, Eq, PartialEq, HashStable_Generic)]
pub enum LitKind {
    /// A string literal (`"foo"`). The symbol is unescaped, and so may differ
    /// from the original token's symbol.
    Str(Symbol, StrStyle),
    /// A byte string (`b"foo"`). Not stored as a symbol because it might be
    /// non-utf8, and symbols only allow utf8 strings.
    ByteStr(Arc<[u8]>, StrStyle),
    /// A C String (`c"foo"`). Guaranteed to only have `\0` at the end.
    CStr(Arc<[u8]>, StrStyle),
    /// A byte char (`b'f'`).
    Byte(u8),
    /// A character literal (`'a'`).
    Char(char),
    /// An integer literal (`1`).
    Int(Pu128, LitIntType),
    /// A float literal (`1.0`, `1f64` or `1E10f64`). The pre-suffix part is
    /// stored as a symbol rather than `f64` so that `LitKind` can impl `Eq`
    /// and `Hash`.
    Float(Symbol, LitFloatType),
    /// A boolean literal (`true`, `false`).
    Bool(bool),
    /// Placeholder for a literal that wasn't well-formed in some way.
    Err(ErrorGuaranteed),
}

impl LitKind {
    pub fn str(&self) -> Option<Symbol> {
        match *self {
            LitKind::Str(s, _) => Some(s),
            _ => None,
        }
    }

    /// Returns `true` if this literal is a string.
    pub fn is_str(&self) -> bool {
        matches!(self, LitKind::Str(..))
    }

    /// Returns `true` if this literal is byte literal string.
    pub fn is_bytestr(&self) -> bool {
        matches!(self, LitKind::ByteStr(..))
    }

    /// Returns `true` if this is a numeric literal.
    pub fn is_numeric(&self) -> bool {
        matches!(self, LitKind::Int(..) | LitKind::Float(..))
    }

    /// Returns `true` if this literal has no suffix.
    /// Note: this will return true for literals with prefixes such as raw strings and byte strings.
    pub fn is_unsuffixed(&self) -> bool {
        !self.is_suffixed()
    }

    /// Returns `true` if this literal has a suffix.
    pub fn is_suffixed(&self) -> bool {
        match *self {
            // suffixed variants
            LitKind::Int(_, LitIntType::Signed(..) | LitIntType::Unsigned(..))
            | LitKind::Float(_, LitFloatType::Suffixed(..)) => true,
            // unsuffixed variants
            LitKind::Str(..)
            | LitKind::ByteStr(..)
            | LitKind::CStr(..)
            | LitKind::Byte(..)
            | LitKind::Char(..)
            | LitKind::Int(_, LitIntType::Unsuffixed)
            | LitKind::Float(_, LitFloatType::Unsuffixed)
            | LitKind::Bool(..)
            | LitKind::Err(_) => false,
        }
    }
}

// N.B., If you change this, you'll probably want to change the corresponding
// type structure in `middle/ty.rs` as well.
#[derive(Clone, Encodable, Decodable, Debug)]
pub struct MutTy {
    pub ty: P<Ty>,
    pub mutbl: Mutability,
}

/// Represents a function's signature in a trait declaration,
/// trait implementation, or free function.
#[derive(Clone, Encodable, Decodable, Debug)]
pub struct FnSig {
    pub header: FnHeader,
    pub decl: P<FnDecl>,
    pub span: Span,
}

#[derive(
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Debug,
    Encodable,
    Decodable,
    HashStable_Generic,
)]
pub enum FloatTy {
    F16,
    F32,
    F64,
    F128,
}

impl FloatTy {
    pub fn name_str(self) -> &'static str {
        match self {
            FloatTy::F16 => "f16",
            FloatTy::F32 => "f32",
            FloatTy::F64 => "f64",
            FloatTy::F128 => "f128",
        }
    }

    pub fn name(self) -> Symbol {
        match self {
            FloatTy::F16 => sym::f16,
            FloatTy::F32 => sym::f32,
            FloatTy::F64 => sym::f64,
            FloatTy::F128 => sym::f128,
        }
    }
}

#[derive(
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Debug,
    Encodable,
    Decodable,
    HashStable_Generic,
)]
pub enum IntTy {
    Isize,
    I8,
    I16,
    I32,
    I64,
    I128,
}

impl IntTy {
    pub fn name_str(&self) -> &'static str {
        match *self {
            IntTy::Isize => "isize",
            IntTy::I8 => "i8",
            IntTy::I16 => "i16",
            IntTy::I32 => "i32",
            IntTy::I64 => "i64",
            IntTy::I128 => "i128",
        }
    }

    pub fn name(&self) -> Symbol {
        match *self {
            IntTy::Isize => sym::isize,
            IntTy::I8 => sym::i8,
            IntTy::I16 => sym::i16,
            IntTy::I32 => sym::i32,
            IntTy::I64 => sym::i64,
            IntTy::I128 => sym::i128,
        }
    }
}

#[derive(
    Clone,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Copy,
    Debug,
    Encodable,
    Decodable,
    HashStable_Generic,
)]
pub enum UintTy {
    Usize,
    U8,
    U16,
    U32,
    U64,
    U128,
}

impl UintTy {
    pub fn name_str(&self) -> &'static str {
        match *self {
            UintTy::Usize => "usize",
            UintTy::U8 => "u8",
            UintTy::U16 => "u16",
            UintTy::U32 => "u32",
            UintTy::U64 => "u64",
            UintTy::U128 => "u128",
        }
    }

    pub fn name(&self) -> Symbol {
        match *self {
            UintTy::Usize => sym::usize,
            UintTy::U8 => sym::u8,
            UintTy::U16 => sym::u16,
            UintTy::U32 => sym::u32,
            UintTy::U64 => sym::u64,
            UintTy::U128 => sym::u128,
        }
    }
}

/// A constraint on an associated item.
///
/// ### Examples
///
/// * the `A = Ty` and `B = Ty` in `Trait<A = Ty, B = Ty>`
/// * the `G<Ty> = Ty` in `Trait<G<Ty> = Ty>`
/// * the `A: Bound` in `Trait<A: Bound>`
/// * the `RetTy` in `Trait(ArgTy, ArgTy) -> RetTy`
/// * the `C = { Ct }` in `Trait<C = { Ct }>` (feature `associated_const_equality`)
/// * the `f(..): Bound` in `Trait<f(..): Bound>` (feature `return_type_notation`)
#[derive(Clone, Encodable, Decodable, Debug)]
pub struct AssocItemConstraint {
    pub id: NodeId,
    pub ident: Ident,
    pub gen_args: Option<GenericArgs>,
    pub kind: AssocItemConstraintKind,
    pub span: Span,
}

#[derive(Clone, Encodable, Decodable, Debug)]
pub enum Term {
    Ty(P<Ty>),
    Const(AnonConst),
}

impl From<P<Ty>> for Term {
    fn from(v: P<Ty>) -> Self {
        Term::Ty(v)
    }
}

impl From<AnonConst> for Term {
    fn from(v: AnonConst) -> Self {
        Term::Const(v)
    }
}

/// The kind of [associated item constraint][AssocItemConstraint].
#[derive(Clone, Encodable, Decodable, Debug)]
pub enum AssocItemConstraintKind {
    /// An equality constraint for an associated item (e.g., `AssocTy = Ty` in `Trait<AssocTy = Ty>`).
    ///
    /// Also known as an *associated item binding* (we *bind* an associated item to a term).
    ///
    /// Furthermore, associated type equality constraints can also be referred to as *associated type
    /// bindings*. Similarly with associated const equality constraints and *associated const bindings*.
    Equality { term: Term },
    /// A bound on an associated type (e.g., `AssocTy: Bound` in `Trait<AssocTy: Bound>`).
    Bound { bounds: GenericBounds },
}

#[derive(Encodable, Decodable, Debug)]
pub struct Ty {
    pub id: NodeId,
    pub kind: TyKind,
    pub span: Span,
    pub tokens: Option<LazyAttrTokenStream>,
}

impl Clone for Ty {
    fn clone(&self) -> Self {
        ensure_sufficient_stack(|| Self {
            id: self.id,
            kind: self.kind.clone(),
            span: self.span,
            tokens: self.tokens.clone(),
        })
    }
}

impl Ty {
    pub fn peel_refs(&self) -> &Self {
        let mut final_ty = self;
        while let TyKind::Ref(_, MutTy { ty, .. }) | TyKind::Ptr(MutTy { ty, .. }) = &final_ty.kind
        {
            final_ty = ty;
        }
        final_ty
    }

    pub fn is_maybe_parenthesised_infer(&self) -> bool {
        match &self.kind {
            TyKind::Infer => true,
            TyKind::Paren(inner) => inner.ast_deref().is_maybe_parenthesised_infer(),
            _ => false,
        }
    }
}

/// The various kinds of type recognized by the compiler.
//
// Adding a new variant? Please update `test_ty` in `tests/ui/macros/stringify.rs`.
#[derive(Clone, Encodable, Decodable, Debug)]
pub enum TyKind {
    /// A variable-length slice (`[T]`).
    Slice(P<Ty>),
    /// A fixed length array (`[T; n]`).
    Array(P<Ty>, AnonConst),
    /// A raw pointer (`*const T` or `*mut T`).
    Ptr(MutTy),
    /// A reference (`&'a T` or `&'a mut T`).
    Ref(Option<Lifetime>, MutTy),
    /// A pinned reference (`&'a pin const T` or `&'a pin mut T`).
    ///
    /// Desugars into `Pin<&'a T>` or `Pin<&'a mut T>`.
    PinnedRef(Option<Lifetime>, MutTy),
    /// The never type (`!`).
    Never,
    /// A tuple (`(A, B, C, D,...)`).
    Tup(ThinVec<P<Ty>>),
    /// A path (`module::module::...::Type`), optionally
    /// "qualified", e.g., `<Vec<T> as SomeTrait>::SomeType`.
    ///
    /// Type parameters are stored in the `Path` itself.
    Path(Option<P<QSelf>>, Path),
    /// A trait object type `Bound1 + Bound2 + Bound3`
    /// where `Bound` is a trait or a lifetime.
    TraitObject(GenericBounds, TraitObjectSyntax),
    /// An `impl Bound1 + Bound2 + Bound3` type
    /// where `Bound` is a trait or a lifetime.
    ///
    /// The `NodeId` exists to prevent lowering from having to
    /// generate `NodeId`s on the fly, which would complicate
    /// the generation of opaque `type Foo = impl Trait` items significantly.
    ImplTrait(NodeId, GenericBounds),
    /// No-op; kept solely so that we can pretty-print faithfully.
    Paren(P<Ty>),
    /// Unused for now.
    Typeof(AnonConst),
    /// This means the type should be inferred instead of it having been
    /// specified. This can appear anywhere in a type.
    Infer,
    /// Inferred type of a `self` or `&self` argument in a method.
    ImplicitSelf,
    /// A macro in the type position.
    MacCall(P<MacCall>),
    /// Placeholder for a `va_list`.
    CVarArgs,
    /// Pattern types like `pattern_type!(u32 is 1..=)`, which is the same as `NonZero<u32>`,
    /// just as part of the type system.
    Pat(P<Ty>, P<TyPat>),
    /// Sometimes we need a dummy value when no error has occurred.
    Dummy,
    /// Placeholder for a kind that has failed to be defined.
    Err(ErrorGuaranteed),
}

impl TyKind {
    pub fn is_implicit_self(&self) -> bool {
        matches!(self, TyKind::ImplicitSelf)
    }

    pub fn is_unit(&self) -> bool {
        matches!(self, TyKind::Tup(tys) if tys.is_empty())
    }

    pub fn is_simple_path(&self) -> Option<Symbol> {
        if let TyKind::Path(None, Path { segments, .. }) = &self
            && let [segment] = &segments[..]
            && segment.args.is_none()
        {
            Some(segment.ident.name)
        } else {
            None
        }
    }
}

/// A pattern type pattern.
#[derive(Clone, Encodable, Decodable, Debug)]
pub struct TyPat {
    pub id: NodeId,
    pub kind: TyPatternKind,
    pub span: Span,
    pub tokens: Option<LazyAttrTokenStream>,
}

/// All the different flavors of pattern that Rust recognizes.
//
// Adding a new variant? Please update `test_pat` in `tests/ui/macros/stringify.rs`.
#[derive(Clone, Encodable, Decodable, Debug)]
pub enum TyPatternKind {
    /// A range pattern (e.g., `1...2`, `1..2`, `1..`, `..2`, `1..=2`, `..=2`).
    Range(
        Option<P<AnonConst>>,
        Option<P<AnonConst>>,
        Spanned<RangeEnd>,
    ),

    /// Placeholder for a pattern that wasn't syntactically well formed in some way.
    Err(ErrorGuaranteed),
}

/// A parameter in a function header.
///
/// E.g., `bar: usize` as in `fn foo(bar: usize)`.
#[derive(Clone, Encodable, Decodable, Debug)]
pub struct Param {
    pub ty: P<Ty>,
    pub pat: P<Pattern>,
    pub id: NodeId,
    pub span: Span,
    pub is_placeholder: bool,
}

/// Alternative representation for `Arg`s describing `self` parameter of methods.
///
/// E.g., `&mut self` as in `fn foo(&mut self)`.
#[derive(Clone, Encodable, Decodable, Debug)]
pub enum SelfKind {
    /// `self`, `mut self`
    Value(Mutability),
    /// `&'lt self`, `&'lt mut self`
    Region(Option<Lifetime>, Mutability),
    /// `&'lt pin const self`, `&'lt pin mut self`
    Pinned(Option<Lifetime>, Mutability),
    /// `self: TYPE`, `mut self: TYPE`
    Explicit(P<Ty>, Mutability),
}

impl SelfKind {
    pub fn to_ref_suggestion(&self) -> String {
        match self {
            SelfKind::Region(None, mutbl) => mutbl.ref_prefix_str().to_string(),
            SelfKind::Region(Some(lt), mutbl) => format!("&{lt} {}", mutbl.prefix_str()),
            SelfKind::Pinned(None, mutbl) => format!("&pin {}", mutbl.ptr_str()),
            SelfKind::Pinned(Some(lt), mutbl) => format!("&{lt} pin {}", mutbl.ptr_str()),
            SelfKind::Value(_) | SelfKind::Explicit(_, _) => {
                unreachable!("if we had an explicit self, we wouldn't be here")
            }
        }
    }
}

pub type ExplicitSelf = Spanned<SelfKind>;

impl Param {
    /// Attempts to cast parameter to `ExplicitSelf`.
    pub fn to_self(&self) -> Option<ExplicitSelf> {
        if let PatternKind::Ident(BindingMode(ByRef::No, mutbl), ident, _) = self.pat.kind {
            if ident.name == kw::SelfLower {
                return match self.ty.kind {
                    TyKind::ImplicitSelf => Some(respan(self.pat.span, SelfKind::Value(mutbl))),
                    TyKind::Ref(lt, MutTy { ref ty, mutbl }) if ty.kind.is_implicit_self() => {
                        Some(respan(self.pat.span, SelfKind::Region(lt, mutbl)))
                    }
                    TyKind::PinnedRef(lt, MutTy { ref ty, mutbl })
                        if ty.kind.is_implicit_self() =>
                    {
                        Some(respan(self.pat.span, SelfKind::Pinned(lt, mutbl)))
                    }
                    _ => Some(respan(
                        self.pat.span.to(self.ty.span),
                        SelfKind::Explicit(self.ty.clone(), mutbl),
                    )),
                };
            }
        }
        None
    }

    /// Returns `true` if parameter is `self`.
    pub fn is_self(&self) -> bool {
        if let PatternKind::Ident(_, ident, _) = self.pat.kind {
            ident.name == kw::SelfLower
        } else {
            false
        }
    }

    /// Builds a `Param` object from `ExplicitSelf`.
    pub fn from_self(attrs: AttrVec, eself: ExplicitSelf, eself_ident: Ident) -> Param {
        let span = eself.span.to(eself_ident.span);
        let infer_ty = P(Ty {
            id: DUMMY_NODE_ID,
            kind: TyKind::ImplicitSelf,
            span: eself_ident.span,
            tokens: None,
        });
        let (mutbl, ty) = match eself.node {
            SelfKind::Explicit(ty, mutbl) => (mutbl, ty),
            SelfKind::Value(mutbl) => (mutbl, infer_ty),
            SelfKind::Region(lt, mutbl) => (
                Mutability::Not,
                P(Ty {
                    id: DUMMY_NODE_ID,
                    kind: TyKind::Ref(
                        lt,
                        MutTy {
                            ty: infer_ty,
                            mutbl,
                        },
                    ),
                    span,
                    tokens: None,
                }),
            ),
            SelfKind::Pinned(lt, mutbl) => (
                mutbl,
                P(Ty {
                    id: DUMMY_NODE_ID,
                    kind: TyKind::PinnedRef(
                        lt,
                        MutTy {
                            ty: infer_ty,
                            mutbl,
                        },
                    ),
                    span,
                    tokens: None,
                }),
            ),
        };
        Param {
            attrs,
            pat: P(Pattern {
                id: DUMMY_NODE_ID,
                kind: PatternKind::Ident(BindingMode(ByRef::No, mutbl), eself_ident, None),
                span,
                tokens: None,
            }),
            span,
            ty,
            id: DUMMY_NODE_ID,
            is_placeholder: false,
        }
    }
}

/// A signature (not the body) of a function declaration.
///
/// E.g., `fn foo(bar: baz)`.
///
/// Please note that it's different from `FnHeader` structure
/// which contains metadata about function safety, asyncness, constness and ABI.
#[derive(Clone, Encodable, Decodable, Debug)]
pub struct FnDecl {
    pub inputs: ThinVec<Param>,
    pub output: FnRetTy,
}

impl FnDecl {
    pub fn has_self(&self) -> bool {
        self.inputs.get(0).is_some_and(Param::is_self)
    }
    pub fn c_variadic(&self) -> bool {
        self.inputs
            .last()
            .is_some_and(|arg| matches!(arg.ty.kind, TyKind::CVarArgs))
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Hash, Encodable, Decodable, Debug, HashStable_Generic)]
pub enum Const {
    Yes(Span),
    No,
}

/// Item defaultness.
/// For details see the [RFC #2532](https://github.com/rust-lang/rfcs/pull/2532).
#[derive(Copy, Clone, PartialEq, Encodable, Decodable, Debug, HashStable_Generic)]
pub enum Defaultness {
    Default(Span),
    Final,
}

#[derive(Clone, Encodable, Decodable, Debug)]
pub enum FnRetTy {
    /// Returns type is not specified.
    ///
    /// Functions default to `()` and closures default to inference.
    /// Span points to where return type would be inserted.
    Default(Span),
    /// Everything else.
    Ty(P<Ty>),
}

impl FnRetTy {
    pub fn span(&self) -> Span {
        match self {
            &FnRetTy::Default(span) => span,
            FnRetTy::Ty(ty) => ty.span,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Encodable, Decodable, Debug)]
pub enum Inline {
    Yes,
    No,
}

/// Module item kind.
#[derive(Clone, Encodable, Decodable, Debug)]
pub enum ModKind {
    /// Module with inlined definition `mod foo { ... }`,
    /// or with definition outlined to a separate file `mod foo;` and already loaded from it.
    /// The inner span is from the first token past `{` to the last token until `}`,
    /// or from the first to the last token in the loaded file.
    Loaded(
        ThinVec<P<Item>>,
        Inline,
        ModSpans,
        Result<(), ErrorGuaranteed>,
    ),
    /// Module with definition outlined to a separate file `mod foo;` but not yet loaded from it.
    Unloaded,
}

#[derive(Copy, Clone, Encodable, Decodable, Debug, Default)]
pub struct ModSpans {
    /// `inner_span` covers the body of the module; for a file module, its the whole file.
    /// For an inline module, its the span inside the `{ ... }`, not including the curly braces.
    pub inner_span: Span,
    pub inject_use_span: Span,
}

#[derive(Clone, Encodable, Decodable, Debug)]
pub struct EnumDef {
    pub variants: ThinVec<Variant>,
}
/// Enum variant.
#[derive(Clone, Encodable, Decodable, Debug)]
pub struct Variant {
    /// Attributes of the variant.
    pub attrs: AttrVec,
    /// Id of the variant (not the constructor, see `VariantData::ctor_id()`).
    pub id: NodeId,
    /// Span
    pub span: Span,
    /// The visibility of the variant. Syntactically accepted but not semantically.
    pub vis: Visibility,
    /// Name of the variant.
    pub ident: Ident,

    /// Fields and constructor id of the variant.
    pub data: VariantData,
    /// Explicit discriminant, e.g., `Foo = 1`.
    pub disr_expr: Option<AnonConst>,
    /// Is a macro placeholder.
    pub is_placeholder: bool,
}

/// Part of `use` item to the right of its prefix.
#[derive(Clone, Encodable, Decodable, Debug)]
pub enum UseTreeKind {
    /// `use prefix` or `use prefix as rename`
    Simple(Option<Ident>),
    /// `use prefix::{...}`
    ///
    /// The span represents the braces of the nested group and all elements within:
    ///
    /// ```text
    /// use foo::{bar, baz};
    ///          ^^^^^^^^^^
    /// ```
    Nested {
        items: ThinVec<(UseTree, NodeId)>,
        span: Span,
    },
    /// `use prefix::*`
    Glob,
}

/// A tree of paths sharing common prefixes.
/// Used in `use` items both at top-level and inside of braces in import groups.
#[derive(Clone, Encodable, Decodable, Debug)]
pub struct UseTree {
    pub prefix: Path,
    pub kind: UseTreeKind,
    pub span: Span,
}

impl UseTree {
    pub fn ident(&self) -> Ident {
        match self.kind {
            UseTreeKind::Simple(Some(rename)) => rename,
            UseTreeKind::Simple(None) => {
                self.prefix
                    .segments
                    .last()
                    .expect("empty prefix in a simple import")
                    .ident
            }
            _ => panic!("`UseTree::ident` can only be used on a simple import"),
        }
    }
}

#[derive(Clone, Encodable, Decodable, Debug)]
pub struct Visibility {
    pub kind: VisibilityKind,
    pub span: Span,
    pub tokens: Option<LazyAttrTokenStream>,
}

#[derive(Clone, Encodable, Decodable, Debug)]
pub enum VisibilityKind {
    Public,
    Restricted {
        path: P<Path>,
        id: NodeId,
        shorthand: bool,
    },
    Inherited,
}

impl VisibilityKind {
    pub fn is_pub(&self) -> bool {
        matches!(self, VisibilityKind::Public)
    }
}

/// Field definition in a struct, variant or union.
///
/// E.g., `bar: usize` as in `struct Foo { bar: usize }`.
#[derive(Clone, Encodable, Decodable, Debug)]
pub struct FieldDef {
    pub attrs: AttrVec,
    pub id: NodeId,
    pub span: Span,
    pub vis: Visibility,
    pub safety: Safety,
    pub ident: Option<Ident>,

    pub ty: P<Ty>,
    pub default: Option<AnonConst>,
    pub is_placeholder: bool,
}

/// Was parsing recovery performed?
#[derive(Copy, Clone, Debug, Encodable, Decodable, HashStable_Generic)]
pub enum Recovered {
    No,
    Yes(ErrorGuaranteed),
}

/// Fields and constructor ids of enum variants and structs.
#[derive(Clone, Encodable, Decodable, Debug)]
pub enum VariantData {
    /// Struct variant.
    ///
    /// E.g., `Bar { .. }` as in `enum Foo { Bar { .. } }`.
    Struct {
        fields: ThinVec<FieldDef>,
        recovered: Recovered,
    },
    /// Tuple variant.
    ///
    /// E.g., `Bar(..)` as in `enum Foo { Bar(..) }`.
    Tuple(ThinVec<FieldDef>, NodeId),
    /// Unit variant.
    ///
    /// E.g., `Bar = ..` as in `enum Foo { Bar = .. }`.
    Unit(NodeId),
}

impl VariantData {
    /// Return the fields of this variant.
    pub fn fields(&self) -> &[FieldDef] {
        match self {
            VariantData::Struct { fields, .. } | VariantData::Tuple(fields, _) => fields,
            _ => &[],
        }
    }

    /// Return the `NodeId` of this variant's constructor, if it has one.
    pub fn ctor_node_id(&self) -> Option<NodeId> {
        match *self {
            VariantData::Struct { .. } => None,
            VariantData::Tuple(_, id) | VariantData::Unit(id) => Some(id),
        }
    }
}

/// An item definition.
#[derive(Clone, Encodable, Decodable, Debug)]
pub struct Item<K = ItemKind> {
    pub id: NodeId,
    pub span: Span,
    pub vis: Visibility,
    /// The name of the item.
    /// It might be a dummy name in case of anonymous items.
    pub ident: Ident,

    pub kind: K,

    /// Original tokens this item was parsed from. This isn't necessarily
    /// available for all items, although over time more and more items should
    /// have this be `Some`. Right now this is primarily used for procedural
    /// macros, notably custom attributes.
    ///
    /// Note that the tokens here do not include the outer attributes, but will
    /// include inner attributes.
    pub tokens: Option<LazyAttrTokenStream>,
}

impl Item {
    pub fn opt_generics(&self) -> Option<&Generics> {
        match &self.kind {
            ItemKind::ExternCrate(_)
            | ItemKind::Use(_)
            | ItemKind::Mod(_, _)
            | ItemKind::ForeignMod(_)
            | ItemKind::GlobalAsm(_)
            | ItemKind::MacCall(_)
            | ItemKind::Delegation(_)
            | ItemKind::DelegationMac(_)
            | ItemKind::MacroDef(_) => None,
            ItemKind::Static(_) => None,
            ItemKind::Const(i) => Some(&i.generics),
            ItemKind::Fn(i) => Some(&i.generics),
            ItemKind::TyAlias(i) => Some(&i.generics),
            ItemKind::TraitAlias(generics, _)
            | ItemKind::Enum(_, generics)
            | ItemKind::Struct(_, generics)
            | ItemKind::Union(_, generics) => Some(&generics),
            ItemKind::Trait(i) => Some(&i.generics),
            ItemKind::Impl(i) => Some(&i.generics),
        }
    }
}

/// A function header.
///
/// All the information between the visibility and the name of the function is
/// included in this struct (e.g., `async unsafe fn` or `const extern "C" fn`).
#[derive(Clone, Copy, Encodable, Decodable, Debug)]
pub struct FnHeader {
    /// The `const` keyword, if any
    pub constness: Const,
}

impl FnHeader {
    /// Does this function header have any qualifiers or is it empty?
    pub fn has_qualifiers(&self) -> bool {
        let Self { constness } = self;
        matches!(constness, Const::Yes(_))
    }
}

impl Default for FnHeader {
    fn default() -> FnHeader {
        FnHeader {
            constness: Const::No,
        }
    }
}

#[derive(Clone, Encodable, Decodable, Debug)]
pub struct TyAlias {
    pub defaultness: Defaultness,
    pub ty: Option<P<Ty>>,
}

#[derive(Clone, Encodable, Decodable, Debug)]
pub struct Impl {
    pub defaultness: Defaultness,
    pub constness: Const,
    pub self_ty: P<Ty>,
    pub items: ThinVec<P<AssocItem>>,
}

#[derive(Clone, Encodable, Decodable, Debug, Default)]
pub struct FnContract {
    pub requires: Option<P<Expr>>,
    pub ensures: Option<P<Expr>>,
}

#[derive(Clone, Encodable, Decodable, Debug)]
pub struct Fn {
    pub defaultness: Defaultness,
    pub generics: Generics,
    pub sig: FnSig,
    pub contract: Option<P<FnContract>>,
    pub define_opaque: Option<ThinVec<(NodeId, Path)>>,
    pub body: Option<P<Block>>,
}

#[derive(Clone, Encodable, Decodable, Debug)]
pub struct Delegation {
    /// Path resolution id.
    pub id: NodeId,
    pub qself: Option<P<QSelf>>,
    pub path: Path,
    pub rename: Option<Ident>,
    pub body: Option<P<Block>>,
    /// The item was expanded from a glob delegation item.
    pub from_glob: bool,
}

#[derive(Clone, Encodable, Decodable, Debug)]
pub struct DelegationMac {
    pub qself: Option<P<QSelf>>,
    pub prefix: Path,
    // Some for list delegation, and None for glob delegation.
    pub suffixes: Option<ThinVec<(Ident, Option<Ident>)>>,
    pub body: Option<P<Block>>,
}

#[derive(Clone, Encodable, Decodable, Debug)]
pub struct StaticItem {
    pub ty: P<Ty>,
    pub safety: Safety,
    pub mutability: Mutability,
    pub expr: Option<P<Expr>>,
}

#[derive(Clone, Encodable, Decodable, Debug)]
pub struct ConstItem {
    pub defaultness: Defaultness,
    pub generics: Generics,
    pub ty: P<Ty>,
    pub expr: Option<P<Expr>>,
}

// Adding a new variant? Please update `test_item` in `tests/ui/macros/stringify.rs`.
#[derive(Clone, Encodable, Decodable, Debug)]
pub enum ItemKind {
    /// A use declaration item (`use`).
    ///
    /// E.g., `use foo;`, `use foo::bar;` or `use foo::bar as FooBar;`.
    Use(UseTree),
    /// A static item (`static`).
    ///
    /// E.g., `static FOO: i32 = 42;` or `static FOO: &'static str = "bar";`.
    Static(Box<StaticItem>),
    /// A constant item (`const`).
    ///
    /// E.g., `const FOO: i32 = 42;`.
    Const(Box<ConstItem>),
    /// A function declaration (`fn`).
    ///
    /// E.g., `fn foo(bar: usize) -> usize { .. }`.
    Fn(Box<Fn>),
    /// A module declaration (`mod`).
    ///
    /// E.g., `mod foo;` or `mod foo { .. }`.
    /// `unsafe` keyword on modules is accepted syntactically for macro DSLs, but not
    /// semantically by Rust.
    Mod(Safety, ModKind),
    /// A type alias (`type`).
    ///
    /// E.g., `type Foo = Bar<u8>;`.
    TyAlias(Box<TyAlias>),
    /// An enum definition (`enum`).
    ///
    /// E.g., `enum Foo<A, B> { C<A>, D<B> }`.
    Enum(EnumDef, Generics),
    /// A struct definition (`struct`).
    ///
    /// E.g., `struct Foo<A> { x: A }`.
    Struct(VariantData, Generics),
    /// An implementation.
    ///
    /// E.g., `impl<A> Foo<A> { .. }` or `impl<A> Trait for Foo<A> { .. }`.
    Impl(Box<Impl>),

    /// A single delegation item (`reuse`).
    ///
    /// E.g. `reuse <Type as Trait>::name { target_expr_template }`.
    Delegation(Box<Delegation>),
}

impl ItemKind {
    /// "a" or "an"
    pub fn article(&self) -> &'static str {
        use ItemKind::*;
        match self {
            Use(..) | Static(..) | Const(..) | Fn(..) | Mod(..) | TyAlias(..) | Struct(..)
            | Delegation(..) => "a",
            Enum(..) | Impl { .. } => "an",
        }
    }

    pub fn descr(&self) -> &'static str {
        match self {
            ItemKind::Use(..) => "`use` import",
            ItemKind::Static(..) => "static item",
            ItemKind::Const(..) => "constant item",
            ItemKind::Fn(..) => "function",
            ItemKind::Mod(..) => "module",
            ItemKind::TyAlias(..) => "type alias",
            ItemKind::Enum(..) => "enum",
            ItemKind::Struct(..) => "struct",
            ItemKind::Impl { .. } => "implementation",
            ItemKind::Delegation(..) => "delegated function",
        }
    }
}

/// Represents associated items.
/// These include items in `impl` and `trait` definitions.
pub type AssocItem = Item<AssocItemKind>;

/// Represents associated item kinds.
///
/// The term "provided" in the variants below refers to the item having a default
/// definition / body. Meanwhile, a "required" item lacks a definition / body.
/// In an implementation, all items must be provided.
/// The `Option`s below denote the bodies, where `Some(_)`
/// means "provided" and conversely `None` means "required".
#[derive(Clone, Encodable, Decodable, Debug)]
pub enum AssocItemKind {
    /// An associated constant, `const $ident: $ty $def?;` where `def ::= "=" $expr? ;`.
    /// If `def` is parsed, then the constant is provided, and otherwise required.
    Const(Box<ConstItem>),
    /// An associated function.
    Fn(Box<Fn>),
    /// An associated type.
    Type(Box<TyAlias>),
    /// An associated delegation item.
    Delegation(Box<Delegation>),
}

impl AssocItemKind {
    pub fn defaultness(&self) -> Defaultness {
        match *self {
            Self::Const(box ConstItem { defaultness, .. })
            | Self::Fn(box Fn { defaultness, .. })
            | Self::Type(box TyAlias { defaultness, .. }) => defaultness,
            Self::Delegation(..) => Defaultness::Final,
        }
    }
}

impl From<AssocItemKind> for ItemKind {
    fn from(assoc_item_kind: AssocItemKind) -> ItemKind {
        match assoc_item_kind {
            AssocItemKind::Const(item) => ItemKind::Const(item),
            AssocItemKind::Fn(fn_kind) => ItemKind::Fn(fn_kind),
            AssocItemKind::Type(ty_alias_kind) => ItemKind::TyAlias(ty_alias_kind),
            AssocItemKind::Delegation(delegation) => ItemKind::Delegation(delegation),
        }
    }
}

impl TryFrom<ItemKind> for AssocItemKind {
    type Error = ItemKind;

    fn try_from(item_kind: ItemKind) -> Result<AssocItemKind, ItemKind> {
        Ok(match item_kind {
            ItemKind::Const(item) => AssocItemKind::Const(item),
            ItemKind::Fn(fn_kind) => AssocItemKind::Fn(fn_kind),
            ItemKind::TyAlias(ty_kind) => AssocItemKind::Type(ty_kind),
            ItemKind::Delegation(d) => AssocItemKind::Delegation(d),
            _ => return Err(item_kind),
        })
    }
}

// Some nodes are used a lot. Make sure they don't unintentionally get bigger.
#[cfg(target_pointer_width = "64")]
mod size_asserts {
    use terebinth_data_structures::static_assert_size;

    use super::*;
    // tidy-alphabetical-start
    static_assert_size!(AssocItem, 88);
    static_assert_size!(AssocItemKind, 16);
    static_assert_size!(Block, 32);
    static_assert_size!(Expr, 72);
    static_assert_size!(ExprKind, 40);
    static_assert_size!(Fn, 176);
    static_assert_size!(Impl, 136);
    static_assert_size!(Item, 136);
    static_assert_size!(ItemKind, 64);
    static_assert_size!(LitKind, 24);
    static_assert_size!(Local, 80);
    static_assert_size!(MetaItemLit, 40);
    static_assert_size!(Param, 40);
    static_assert_size!(Pattern, 72);
    static_assert_size!(Path, 24);
    static_assert_size!(PathSegment, 24);
    static_assert_size!(PatternKind, 48);
    static_assert_size!(Stmt, 32);
    static_assert_size!(StmtKind, 16);
    static_assert_size!(Ty, 64);
    static_assert_size!(TyKind, 40);
    // tidy-alphabetical-end
}
