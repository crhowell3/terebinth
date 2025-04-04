use std::ops::DerefMut;
use std::panic;
use std::sync::Arc;

use smallvec::{Array, SmallVec, smallvec};
use terebinth_data_structures::{
    flat_map_in_place::FlatMapInPlace, stack::ensure_sufficient_stack,
};
use terebinth_span::{Ident, Span, source_map::Spanned};
use thin_vec::ThinVec;

use crate::ast::*;
use crate::token::{self, Token};
use crate::visit::{AssocCtx, BoundKind, FnCtx};

pub trait ExpectOne<A: Array> {
    fn expect_one(self, err: &'static str) -> A::Item;
}

impl<A: Array> ExpectOne<A> for SmallVec<A> {
    fn expect_one(self, err: &'static str) -> A::Item {
        assert!(self.len() == 1, "{}", err);
        self.into_iter().next().unwrap()
    }
}

pub trait WalkItemKind {
    type Ctx;
    fn walk(
        &mut self,
        span: Span,
        id: NodeId,
        ident: &mut Ident,
        visibility: &mut Visibility,
        ctx: Self::Ctx,
        visitor: &mut impl MutVisitor,
    );
}

pub trait MutVisitor: Sized {
    /// Mutable token visiting only exists for the `macro_rules` token marker and should not be
    /// used otherwise. Token visitor would be entirely separate from the regular visitor if
    /// the marker didn't have to visit AST fragments in nonterminal tokens.
    const VISIT_TOKENS: bool = false;

    // Methods in this trait have one of three forms:
    //
    //   fn visit_t(&mut self, t: &mut T);                      // common
    //   fn flat_map_t(&mut self, t: T) -> SmallVec<[T; 1]>;    // rare
    //   fn filter_map_t(&mut self, t: T) -> Option<T>;         // rarest
    //
    // Any additions to this trait should happen in form of a call to a public
    // `noop_*` function that only calls out to the visitor again, not other
    // `noop_*` functions. This is a necessary API workaround to the problem of
    // not being able to call out to the super default method in an overridden
    // default method.
    //
    // When writing these methods, it is better to use destructuring like this:
    //
    //   fn visit_abc(&mut self, ABC { a, b, c: _ }: &mut ABC) {
    //       visit_a(a);
    //       visit_b(b);
    //   }
    //
    // than to use field access like this:
    //
    //   fn visit_abc(&mut self, abc: &mut ABC) {
    //       visit_a(&mut abc.a);
    //       visit_b(&mut abc.b);
    //       // ignore abc.c
    //   }
    //
    // As well as being more concise, the former is explicit about which fields
    // are skipped. Furthermore, if a new field is added, the destructuring
    // version will cause a compile error, which is good. In comparison, the
    // field access version will continue working and it would be easy to
    // forget to add handling for it.

    fn visit_use_tree(&mut self, use_tree: &mut UseTree) {
        walk_use_tree(self, use_tree);
    }

    fn visit_item(&mut self, i: &mut P<Item>) {
        walk_item(self, i);
    }

    fn flat_map_item(&mut self, i: P<Item>) -> SmallVec<[P<Item>; 1]> {
        walk_flat_map_item(self, i)
    }

    fn visit_fn_header(&mut self, header: &mut FnHeader) {
        walk_fn_header(self, header);
    }

    fn visit_field_def(&mut self, fd: &mut FieldDef) {
        walk_field_def(self, fd);
    }

    fn flat_map_field_def(&mut self, fd: FieldDef) -> SmallVec<[FieldDef; 1]> {
        walk_flat_map_field_def(self, fd)
    }

    fn visit_assoc_item(&mut self, i: &mut P<AssocItem>, ctxt: AssocCtxt) {
        walk_assoc_item(self, i, ctxt)
    }

    fn flat_map_assoc_item(
        &mut self,
        i: P<AssocItem>,
        ctxt: AssocCtxt,
    ) -> SmallVec<[P<AssocItem>; 1]> {
        walk_flat_map_assoc_item(self, i, ctxt)
    }

    fn visit_contract(&mut self, c: &mut P<FnContract>) {
        walk_contract(self, c);
    }

    fn visit_fn_decl(&mut self, d: &mut P<FnDecl>) {
        walk_fn_decl(self, d);
    }

    /// `Span` and `NodeId` are mutated at the caller site.
    fn visit_fn(&mut self, fk: FnKind<'_>, _: Span, _: NodeId) {
        walk_fn(self, fk)
    }

    fn visit_closure_binder(&mut self, b: &mut ClosureBinder) {
        walk_closure_binder(self, b);
    }

    fn visit_block(&mut self, b: &mut P<Block>) {
        walk_block(self, b);
    }

    fn flat_map_stmt(&mut self, s: Stmt) -> SmallVec<[Stmt; 1]> {
        walk_flat_map_stmt(self, s)
    }

    fn visit_arm(&mut self, arm: &mut Arm) {
        walk_arm(self, arm);
    }

    fn flat_map_arm(&mut self, arm: Arm) -> SmallVec<[Arm; 1]> {
        walk_flat_map_arm(self, arm)
    }

    fn visit_pat(&mut self, p: &mut P<Pattern>) {
        walk_pat(self, p);
    }

    fn visit_anon_const(&mut self, c: &mut AnonConst) {
        walk_anon_const(self, c);
    }

    fn visit_expr(&mut self, e: &mut P<Expr>) {
        walk_expr(self, e);
    }

    /// This method is a hack to workaround unstable of `stmt_expr_attributes`.
    /// It can be removed once that feature is stabilized.
    fn visit_method_receiver_expr(&mut self, ex: &mut P<Expr>) {
        self.visit_expr(ex)
    }

    fn filter_map_expr(&mut self, e: P<Expr>) -> Option<P<Expr>> {
        noop_filter_map_expr(self, e)
    }

    fn visit_ty(&mut self, t: &mut P<Ty>) {
        walk_ty(self, t);
    }

    fn visit_ty_pat(&mut self, t: &mut P<TyPat>) {
        walk_ty_pat(self, t);
    }

    fn visit_assoc_item_constraint(&mut self, c: &mut AssocItemConstraint) {
        walk_assoc_item_constraint(self, c);
    }

    fn visit_variant(&mut self, v: &mut Variant) {
        walk_variant(self, v);
    }

    fn flat_map_variant(&mut self, v: Variant) -> SmallVec<[Variant; 1]> {
        walk_flat_map_variant(self, v)
    }

    fn visit_ident(&mut self, i: &mut Ident) {
        walk_ident(self, i);
    }

    fn visit_path(&mut self, p: &mut Path) {
        walk_path(self, p);
    }

    fn visit_path_segment(&mut self, p: &mut PathSegment) {
        walk_path_segment(self, p)
    }

    fn visit_qself(&mut self, qs: &mut Option<P<QSelf>>) {
        walk_qself(self, qs);
    }

    fn visit_local(&mut self, l: &mut P<Local>) {
        walk_local(self, l);
    }

    fn visit_label(&mut self, label: &mut Label) {
        walk_label(self, label);
    }

    fn visit_param(&mut self, param: &mut Param) {
        walk_param(self, param);
    }

    fn flat_map_param(&mut self, param: Param) -> SmallVec<[Param; 1]> {
        walk_flat_map_param(self, param)
    }

    fn visit_variant_data(&mut self, vdata: &mut VariantData) {
        walk_variant_data(self, vdata);
    }

    fn flat_map_generic_param(&mut self, param: GenericParam) -> SmallVec<[GenericParam; 1]> {
        walk_flat_map_generic_param(self, param)
    }

    fn visit_param_bound(&mut self, tpb: &mut GenericBound, _ctxt: BoundKind) {
        walk_param_bound(self, tpb);
    }

    fn visit_precise_capturing_arg(&mut self, arg: &mut PreciseCapturingArg) {
        walk_precise_capturing_arg(self, arg);
    }

    fn visit_mt(&mut self, mt: &mut MutTy) {
        walk_mt(self, mt);
    }

    fn visit_expr_field(&mut self, f: &mut ExprField) {
        walk_expr_field(self, f);
    }

    fn flat_map_expr_field(&mut self, f: ExprField) -> SmallVec<[ExprField; 1]> {
        walk_flat_map_expr_field(self, f)
    }

    fn visit_vis(&mut self, vis: &mut Visibility) {
        walk_vis(self, vis);
    }

    fn visit_id(&mut self, _id: &mut NodeId) {
        // Do nothing.
    }

    fn visit_span(&mut self, _sp: &mut Span) {
        // Do nothing.
    }

    fn visit_pat_field(&mut self, fp: &mut PatternField) {
        walk_pat_field(self, fp)
    }

    fn flat_map_pat_field(&mut self, fp: PatternField) -> SmallVec<[PatternField; 1]> {
        walk_flat_map_pat_field(self, fp)
    }

    fn visit_format_args(&mut self, fmt: &mut FormatArgs) {
        walk_format_args(self, fmt)
    }

    fn visit_capture_by(&mut self, capture_by: &mut CaptureBy) {
        walk_capture_by(self, capture_by)
    }

    fn visit_fn_ret_ty(&mut self, fn_ret_ty: &mut FnRetTy) {
        walk_fn_ret_ty(self, fn_ret_ty)
    }
}

/// Use a map-style function (`FnOnce(T) -> T`) to overwrite a `&mut T`. Useful
/// when using a `flat_map_*` or `filter_map_*` method within a `visit_`
/// method.
//
// No `noop_` prefix because there isn't a corresponding method in `MutVisitor`.
pub fn visit_clobber<T: DummyAstNode>(t: &mut T, f: impl FnOnce(T) -> T) {
    let old_t = std::mem::replace(t, T::dummy());
    *t = f(old_t);
}

// No `noop_` prefix because there isn't a corresponding method in `MutVisitor`.
#[inline]
fn visit_vec<T, F>(elems: &mut Vec<T>, mut visit_elem: F)
where
    F: FnMut(&mut T),
{
    for elem in elems {
        visit_elem(elem);
    }
}

// No `noop_` prefix because there isn't a corresponding method in `MutVisitor`.
#[inline]
fn visit_thin_vec<T, F>(elems: &mut ThinVec<T>, mut visit_elem: F)
where
    F: FnMut(&mut T),
{
    for elem in elems {
        visit_elem(elem);
    }
}

// No `noop_` prefix because there isn't a corresponding method in `MutVisitor`.
#[inline]
fn visit_opt<T, F>(opt: &mut Option<T>, mut visit_elem: F)
where
    F: FnMut(&mut T),
{
    if let Some(elem) = opt {
        visit_elem(elem);
    }
}

// No `noop_` prefix because there isn't a corresponding method in `MutVisitor`.
#[allow(unused)]
fn visit_exprs<T: MutVisitor>(vis: &mut T, exprs: &mut Vec<P<Expr>>) {
    exprs.flat_map_in_place(|expr| vis.filter_map_expr(expr))
}

// No `noop_` prefix because there isn't a corresponding method in `MutVisitor`.
fn visit_thin_exprs<T: MutVisitor>(vis: &mut T, exprs: &mut ThinVec<P<Expr>>) {
    exprs.flat_map_in_place(|expr| vis.filter_map_expr(expr))
}

// No `noop_` prefix because there isn't a corresponding method in `MutVisitor`.
fn visit_delim_args<T: MutVisitor>(vis: &mut T, args: &mut DelimArgs) {
    let DelimArgs {
        dspan,
        delim: _,
        tokens,
    } = args;
    visit_tts(vis, tokens);
    visit_delim_span(vis, dspan);
}

pub fn visit_delim_span<T: MutVisitor>(vis: &mut T, DelimSpan { open, close }: &mut DelimSpan) {
    vis.visit_span(open);
    vis.visit_span(close);
}

pub fn walk_pat_field<T: MutVisitor>(vis: &mut T, fp: &mut PatternField) {
    let PatternField {
        id,
        ident,
        is_placeholder: _,
        is_shorthand: _,
        pattern: pat,
        span,
    } = fp;
    vis.visit_id(id);
    vis.visit_ident(ident);
    vis.visit_pat(pat);
    vis.visit_span(span);
}

pub fn walk_flat_map_pat_field<T: MutVisitor>(
    vis: &mut T,
    mut fp: PatternField,
) -> SmallVec<[PatternField; 1]> {
    vis.visit_pat_field(&mut fp);
    smallvec![fp]
}

fn walk_use_tree<T: MutVisitor>(vis: &mut T, use_tree: &mut UseTree) {
    let UseTree { prefix, kind, span } = use_tree;
    vis.visit_path(prefix);
    match kind {
        UseTreeKind::Simple(rename) => visit_opt(rename, |rename| vis.visit_ident(rename)),
        UseTreeKind::Nested { items, span } => {
            for (tree, id) in items {
                vis.visit_id(id);
                vis.visit_use_tree(tree);
            }
            vis.visit_span(span);
        }
        UseTreeKind::Glob => {}
    }
    vis.visit_span(span);
}

pub fn walk_arm<T: MutVisitor>(vis: &mut T, arm: &mut Arm) {
    let Arm {
        attrs,
        pat,
        guard,
        body,
        span,
        id,
        is_placeholder: _,
    } = arm;
    vis.visit_id(id);
    visit_attrs(vis, attrs);
    vis.visit_pat(pat);
    visit_opt(guard, |guard| vis.visit_expr(guard));
    visit_opt(body, |body| vis.visit_expr(body));
    vis.visit_span(span);
}

pub fn walk_flat_map_arm<T: MutVisitor>(vis: &mut T, mut arm: Arm) -> SmallVec<[Arm; 1]> {
    vis.visit_arm(&mut arm);
    smallvec![arm]
}

fn walk_assoc_item_constraint<T: MutVisitor>(
    vis: &mut T,
    AssocItemConstraint {
        id,
        ident,
        gen_args,
        kind,
        span,
    }: &mut AssocItemConstraint,
) {
    vis.visit_id(id);
    vis.visit_ident(ident);
    if let Some(gen_args) = gen_args {
        vis.visit_generic_args(gen_args);
    }
    match kind {
        AssocItemConstraintKind::Equality { term } => match term {
            Term::Ty(ty) => vis.visit_ty(ty),
            Term::Const(c) => vis.visit_anon_const(c),
        },
        AssocItemConstraintKind::Bound { bounds } => visit_bounds(vis, bounds, BoundKind::Bound),
    }
    vis.visit_span(span);
}

pub fn walk_ty<T: MutVisitor>(vis: &mut T, ty: &mut P<Ty>) {
    let Ty {
        id,
        kind,
        span,
        tokens,
    } = ty.deref_mut();
    vis.visit_id(id);
    match kind {
        TyKind::Err(_guar) => {}
        TyKind::Infer | TyKind::ImplicitSelf | TyKind::Dummy | TyKind::Never | TyKind::CVarArgs => {
        }
        TyKind::Slice(ty) => vis.visit_ty(ty),
        TyKind::Ptr(mt) => vis.visit_mt(mt),
        TyKind::Ref(lt, mt) | TyKind::PinnedRef(lt, mt) => {
            visit_opt(lt, |lt| vis.visit_lifetime(lt));
            vis.visit_mt(mt);
        }
        TyKind::BareFn(bft) => {
            let BareFnTy {
                safety,
                ext: _,
                generic_params,
                decl,
                decl_span,
            } = bft.deref_mut();
            visit_safety(vis, safety);
            generic_params.flat_map_in_place(|param| vis.flat_map_generic_param(param));
            vis.visit_fn_decl(decl);
            vis.visit_span(decl_span);
        }
        TyKind::UnsafeBinder(binder) => {
            let UnsafeBinderTy {
                generic_params,
                inner_ty,
            } = binder.deref_mut();
            generic_params.flat_map_in_place(|param| vis.flat_map_generic_param(param));
            vis.visit_ty(inner_ty);
        }
        TyKind::Tup(tys) => visit_thin_vec(tys, |ty| vis.visit_ty(ty)),
        TyKind::Paren(ty) => vis.visit_ty(ty),
        TyKind::Pat(ty, pat) => {
            vis.visit_ty(ty);
            vis.visit_ty_pat(pat);
        }
        TyKind::Path(qself, path) => {
            vis.visit_qself(qself);
            vis.visit_path(path);
        }
        TyKind::Array(ty, length) => {
            vis.visit_ty(ty);
            vis.visit_anon_const(length);
        }
        TyKind::Typeof(expr) => vis.visit_anon_const(expr),
        TyKind::TraitObject(bounds, _syntax) => visit_vec(bounds, |bound| {
            vis.visit_param_bound(bound, BoundKind::TraitObject)
        }),
        TyKind::ImplTrait(id, bounds) => {
            vis.visit_id(id);
            visit_vec(bounds, |bound| {
                vis.visit_param_bound(bound, BoundKind::Impl)
            });
        }
        TyKind::MacCall(mac) => vis.visit_mac_call(mac),
    }
    visit_lazy_tts(vis, tokens);
    vis.visit_span(span);
}

pub fn walk_ty_pat<T: MutVisitor>(vis: &mut T, ty: &mut P<TyPat>) {
    let TyPat {
        id,
        kind,
        span,
        tokens,
    } = ty.deref_mut();
    vis.visit_id(id);
    match kind {
        TyPatternKind::Range(start, end, _include_end) => {
            visit_opt(start, |c| vis.visit_anon_const(c));
            visit_opt(end, |c| vis.visit_anon_const(c));
        }
        TyPatternKind::Err(_) => {}
    }
    visit_lazy_tts(vis, tokens);
    vis.visit_span(span);
}

fn walk_foreign_mod<T: MutVisitor>(vis: &mut T, foreign_mod: &mut ForeignMod) {
    let ForeignMod {
        extern_span: _,
        safety,
        abi: _,
        items,
    } = foreign_mod;
    visit_safety(vis, safety);
    items.flat_map_in_place(|item| vis.flat_map_foreign_item(item));
}

pub fn walk_variant<T: MutVisitor>(visitor: &mut T, variant: &mut Variant) {
    let Variant {
        ident,
        vis,
        attrs,
        id,
        data,
        disr_expr,
        span,
        is_placeholder: _,
    } = variant;
    visitor.visit_id(id);
    visit_attrs(visitor, attrs);
    visitor.visit_vis(vis);
    visitor.visit_ident(ident);
    visitor.visit_variant_data(data);
    visit_opt(disr_expr, |disr_expr| visitor.visit_anon_const(disr_expr));
    visitor.visit_span(span);
}

pub fn walk_flat_map_variant<T: MutVisitor>(
    vis: &mut T,
    mut variant: Variant,
) -> SmallVec<[Variant; 1]> {
    vis.visit_variant(&mut variant);
    smallvec![variant]
}

fn walk_ident<T: MutVisitor>(vis: &mut T, Ident { name: _, span }: &mut Ident) {
    vis.visit_span(span);
}

fn walk_path_segment<T: MutVisitor>(vis: &mut T, segment: &mut PathSegment) {
    let PathSegment { ident, id, args } = segment;
    vis.visit_id(id);
    vis.visit_ident(ident);
    visit_opt(args, |args| vis.visit_generic_args(args));
}

fn walk_path<T: MutVisitor>(
    vis: &mut T,
    Path {
        segments,
        span,
        tokens,
    }: &mut Path,
) {
    for segment in segments {
        vis.visit_path_segment(segment);
    }
    visit_lazy_tts(vis, tokens);
    vis.visit_span(span);
}

fn walk_qself<T: MutVisitor>(vis: &mut T, qself: &mut Option<P<QSelf>>) {
    visit_opt(qself, |qself| {
        let QSelf {
            ty,
            path_span,
            position: _,
        } = &mut **qself;
        vis.visit_ty(ty);
        vis.visit_span(path_span);
    })
}

fn walk_local<T: MutVisitor>(vis: &mut T, local: &mut P<Local>) {
    let Local {
        id,
        pat,
        ty,
        kind,
        span,
        colon_sp,
        tokens,
    } = local.deref_mut();
    vis.visit_id(id);
    visit_attrs(vis, attrs);
    vis.visit_pat(pat);
    visit_opt(ty, |ty| vis.visit_ty(ty));
    match kind {
        LocalKind::Decl => {}
        LocalKind::Init(init) => {
            vis.visit_expr(init);
        }
        LocalKind::InitElse(init, els) => {
            vis.visit_expr(init);
            vis.visit_block(els);
        }
    }
    visit_lazy_tts(vis, tokens);
    visit_opt(colon_sp, |sp| vis.visit_span(sp));
    vis.visit_span(span);
}

pub fn walk_param<T: MutVisitor>(vis: &mut T, param: &mut Param) {
    let Param {
        id,
        pat,
        span,
        ty,
        is_placeholder: _,
    } = param;
    vis.visit_id(id);
    vis.visit_pat(pat);
    vis.visit_ty(ty);
    vis.visit_span(span);
}

pub fn walk_flat_map_param<T: MutVisitor>(vis: &mut T, mut param: Param) -> SmallVec<[Param; 1]> {
    vis.visit_param(&mut param);
    smallvec![param]
}

// No `noop_` prefix because there isn't a corresponding method in `MutVisitor`.
fn visit_tt<T: MutVisitor>(vis: &mut T, tt: &mut TokenTree) {
    match tt {
        TokenTree::Token(token, _spacing) => {
            visit_token(vis, token);
        }
        TokenTree::Delimited(dspan, _spacing, _delim, tts) => {
            visit_tts(vis, tts);
            visit_delim_span(vis, dspan);
        }
    }
}

// No `noop_` prefix because there isn't a corresponding method in `MutVisitor`.
fn visit_tts<T: MutVisitor>(vis: &mut T, TokenStream(tts): &mut TokenStream) {
    if T::VISIT_TOKENS && !tts.is_empty() {
        let tts = Arc::make_mut(tts);
        visit_vec(tts, |tree| visit_tt(vis, tree));
    }
}

fn visit_lazy_tts_opt_mut<T: MutVisitor>(vis: &mut T, lazy_tts: Option<&mut LazyAttrTokenStream>) {
    if T::VISIT_TOKENS {
        if let Some(lazy_tts) = lazy_tts {
            let mut tts = lazy_tts.to_attr_token_stream();
            visit_attr_tts(vis, &mut tts);
            *lazy_tts = LazyAttrTokenStream::new(tts);
        }
    }
}

fn visit_lazy_tts<T: MutVisitor>(vis: &mut T, lazy_tts: &mut Option<LazyAttrTokenStream>) {
    visit_lazy_tts_opt_mut(vis, lazy_tts.as_mut());
}

/// Applies ident visitor if it's an ident; applies other visits to interpolated nodes.
/// In practice the ident part is not actually used by specific visitors right now,
/// but there's a test below checking that it works.
// No `noop_` prefix because there isn't a corresponding method in `MutVisitor`.
pub fn visit_token<T: MutVisitor>(vis: &mut T, t: &mut Token) {
    let Token { kind, span } = t;
    match kind {
        token::Ident(name, _is_raw) | token::Lifetime(name, _is_raw) => {
            let mut ident = Ident::new(*name, *span);
            vis.visit_ident(&mut ident);
            *name = ident.name;
            *span = ident.span;
            return; // Avoid visiting the span for the second time.
        }
        token::NtIdent(ident, _is_raw) => {
            vis.visit_ident(ident);
        }
        token::NtLifetime(ident, _is_raw) => {
            vis.visit_ident(ident);
        }
        token::Interpolated(nt) => {
            let nt = Arc::make_mut(nt);
            visit_nonterminal(vis, nt);
        }
        _ => {}
    }
    vis.visit_span(span);
}

// No `noop_` prefix because there isn't a corresponding method in `MutVisitor`.
/// Applies the visitor to elements of interpolated nodes.
//
// N.B., this can occur only when applying a visitor to partially expanded
// code, where parsed pieces have gotten implanted ito *other* macro
// invocations. This is relevant for macro hygiene, but possibly not elsewhere.
//
// One problem here occurs because the types for flat_map_item, flat_map_stmt,
// etc., allow the visitor to return *multiple* items; this is a problem for the
// nodes here, because they insist on having exactly one piece. One solution
// would be to mangle the MutVisitor trait to include one-to-many and
// one-to-one versions of these entry points, but that would probably confuse a
// lot of people and help very few. Instead, I'm just going to put in dynamic
// checks. I think the performance impact of this will be pretty much
// nonexistent. The danger is that someone will apply a `MutVisitor` to a
// partially expanded node, and will be confused by the fact that their
// `flat_map_item` or `flat_map_stmt` isn't getting called on `NtItem` or `NtStmt`
// nodes. Hopefully they'll wind up reading this comment, and doing something
// appropriate.
//
// BTW, design choice: I considered just changing the type of, e.g., `NtItem` to
// contain multiple items, but decided against it when I looked at
// `parse_item_or_view_item` and tried to figure out what I would do with
// multiple items there....
fn visit_nonterminal<T: MutVisitor>(vis: &mut T, nt: &mut token::Nonterminal) {
    match nt {
        token::NtBlock(block) => vis.visit_block(block),
        token::NtExpr(expr) => vis.visit_expr(expr),
        token::NtLiteral(expr) => vis.visit_expr(expr),
    }
}

// No `noop_` prefix because there isn't a corresponding method in `MutVisitor`.
fn visit_defaultness<T: MutVisitor>(vis: &mut T, defaultness: &mut Defaultness) {
    match defaultness {
        Defaultness::Default(span) => vis.visit_span(span),
        Defaultness::Final => {}
    }
}

// No `noop_` prefix because there isn't a corresponding method in `MutVisitor`.
fn visit_constness<T: MutVisitor>(vis: &mut T, constness: &mut Const) {
    match constness {
        Const::Yes(span) => vis.visit_span(span),
        Const::No => {}
    }
}

fn walk_closure_binder<T: MutVisitor>(vis: &mut T, binder: &mut ClosureBinder) {
    match binder {
        ClosureBinder::NotPresent => {}
        ClosureBinder::For {
            span: _,
            generic_params,
        } => {
            generic_params.flat_map_in_place(|param| vis.flat_map_generic_param(param));
        }
    }
}

fn walk_fn<T: MutVisitor>(vis: &mut T, kind: FnKind<'_>) {
    match kind {
        FnKind::Fn(
            _ctxt,
            _ident,
            _vis,
            Fn {
                defaultness,
                generics,
                contract,
                body,
                sig: FnSig { header, decl, span },
                define_opaque,
            },
        ) => {
            // Identifier and visibility are visited as a part of the item.
            visit_defaultness(vis, defaultness);
            vis.visit_fn_header(header);
            vis.visit_generics(generics);
            vis.visit_fn_decl(decl);
            if let Some(contract) = contract {
                vis.visit_contract(contract);
            }
            if let Some(body) = body {
                vis.visit_block(body);
            }
            vis.visit_span(span);

            for (id, path) in define_opaque.iter_mut().flatten() {
                vis.visit_id(id);
                vis.visit_path(path)
            }
        }
        FnKind::Closure(binder, coroutine_kind, decl, body) => {
            vis.visit_closure_binder(binder);
            coroutine_kind
                .as_mut()
                .map(|coroutine_kind| vis.visit_coroutine_kind(coroutine_kind));
            vis.visit_fn_decl(decl);
            vis.visit_expr(body);
        }
    }
}

fn walk_contract<T: MutVisitor>(vis: &mut T, contract: &mut P<FnContract>) {
    let FnContract { requires, ensures } = contract.deref_mut();
    if let Some(pred) = requires {
        vis.visit_expr(pred);
    }
    if let Some(pred) = ensures {
        vis.visit_expr(pred);
    }
}

fn walk_fn_decl<T: MutVisitor>(vis: &mut T, decl: &mut P<FnDecl>) {
    let FnDecl { inputs, output } = decl.deref_mut();
    inputs.flat_map_in_place(|param| vis.flat_map_param(param));
    vis.visit_fn_ret_ty(output);
}

fn walk_fn_ret_ty<T: MutVisitor>(vis: &mut T, fn_ret_ty: &mut FnRetTy) {
    match fn_ret_ty {
        FnRetTy::Default(span) => vis.visit_span(span),
        FnRetTy::Ty(ty) => vis.visit_ty(ty),
    }
}

fn walk_label<T: MutVisitor>(vis: &mut T, Label { ident }: &mut Label) {
    vis.visit_ident(ident);
}

fn walk_variant_data<T: MutVisitor>(vis: &mut T, vdata: &mut VariantData) {
    match vdata {
        VariantData::Struct {
            fields,
            recovered: _,
        } => {
            fields.flat_map_in_place(|field| vis.flat_map_field_def(field));
        }
        VariantData::Tuple(fields, id) => {
            vis.visit_id(id);
            fields.flat_map_in_place(|field| vis.flat_map_field_def(field));
        }
        VariantData::Unit(id) => vis.visit_id(id),
    }
}

pub fn walk_field_def<T: MutVisitor>(visitor: &mut T, fd: &mut FieldDef) {
    let FieldDef {
        span,
        ident,
        vis,
        id,
        ty,
        attrs,
        is_placeholder: _,
        safety,
        default,
    } = fd;
    visitor.visit_id(id);
    visit_attrs(visitor, attrs);
    visitor.visit_vis(vis);
    visit_safety(visitor, safety);
    visit_opt(ident, |ident| visitor.visit_ident(ident));
    visitor.visit_ty(ty);
    visit_opt(default, |default| visitor.visit_anon_const(default));
    visitor.visit_span(span);
}

pub fn walk_flat_map_field_def<T: MutVisitor>(
    vis: &mut T,
    mut fd: FieldDef,
) -> SmallVec<[FieldDef; 1]> {
    vis.visit_field_def(&mut fd);
    smallvec![fd]
}

pub fn walk_expr_field<T: MutVisitor>(vis: &mut T, f: &mut ExprField) {
    let ExprField {
        ident,
        expr,
        span,
        is_shorthand: _,
        attrs,
        id,
        is_placeholder: _,
    } = f;
    vis.visit_id(id);
    visit_attrs(vis, attrs);
    vis.visit_ident(ident);
    vis.visit_expr(expr);
    vis.visit_span(span);
}

pub fn walk_flat_map_expr_field<T: MutVisitor>(
    vis: &mut T,
    mut f: ExprField,
) -> SmallVec<[ExprField; 1]> {
    vis.visit_expr_field(&mut f);
    smallvec![f]
}

fn walk_mt<T: MutVisitor>(vis: &mut T, MutTy { ty, mutbl: _ }: &mut MutTy) {
    vis.visit_ty(ty);
}

pub fn walk_block<T: MutVisitor>(vis: &mut T, block: &mut P<Block>) {
    let Block {
        id,
        stmts,
        rules: _,
        span,
        tokens,
        could_be_bare_literal: _,
    } = block.deref_mut();
    vis.visit_id(id);
    stmts.flat_map_in_place(|stmt| vis.flat_map_stmt(stmt));
    visit_lazy_tts(vis, tokens);
    vis.visit_span(span);
}

pub fn walk_item_kind<K: WalkItemKind>(
    kind: &mut K,
    span: Span,
    id: NodeId,
    ident: &mut Ident,
    visibility: &mut Visibility,
    ctxt: K::Ctxt,
    vis: &mut impl MutVisitor,
) {
    kind.walk(span, id, ident, visibility, ctxt, vis)
}

impl WalkItemKind for ItemKind {
    type Ctx = ();
    fn walk(
        &mut self,
        span: Span,
        id: NodeId,
        ident: &mut Ident,
        visibility: &mut Visibility,
        _ctxt: Self::Ctxt,
        vis: &mut impl MutVisitor,
    ) {
        match self {
            ItemKind::Use(use_tree) => vis.visit_use_tree(use_tree),
            ItemKind::Static(box StaticItem {
                ty,
                safety: _,
                mutability: _,
                expr,
            }) => {
                vis.visit_ty(ty);
                visit_opt(expr, |expr| vis.visit_expr(expr));
            }
            ItemKind::Const(item) => {
                visit_const_item(item, vis);
            }
            ItemKind::Fn(func) => {
                vis.visit_fn(
                    FnKind::Fn(FnCtxt::Free, ident, visibility, &mut *func),
                    span,
                    id,
                );
            }
            ItemKind::Mod(safety, mod_kind) => {
                visit_safety(vis, safety);
                match mod_kind {
                    ModKind::Loaded(
                        items,
                        _inline,
                        ModSpans {
                            inner_span,
                            inject_use_span,
                        },
                        _,
                    ) => {
                        items.flat_map_in_place(|item| vis.flat_map_item(item));
                        vis.visit_span(inner_span);
                        vis.visit_span(inject_use_span);
                    }
                    ModKind::Unloaded => {}
                }
            }
            ItemKind::TyAlias(box TyAlias { defaultness, ty }) => {
                visit_defaultness(vis, defaultness);
                vis.visit_generics(generics);
                visit_bounds(vis, bounds, BoundKind::Bound);
                visit_opt(ty, |ty| vis.visit_ty(ty));
                walk_ty_alias_where_clauses(vis, where_clauses);
            }
            ItemKind::Enum(EnumDef { variants }, generics) => {
                vis.visit_generics(generics);
                variants.flat_map_in_place(|variant| vis.flat_map_variant(variant));
            }
            ItemKind::Struct(variant_data, generics) | ItemKind::Union(variant_data, generics) => {
                vis.visit_generics(generics);
                vis.visit_variant_data(variant_data);
            }
            ItemKind::Impl(box Impl {
                defaultness,
                constness,
                self_ty,
                items,
            }) => {
                visit_defaultness(vis, defaultness);
                visit_constness(vis, constness);
                vis.visit_ty(self_ty);
                items.flat_map_in_place(|item| vis.flat_map_assoc_item(item, AssocCtxt::Impl));
            }
            ItemKind::Delegation(box Delegation {
                id,
                qself,
                path,
                rename,
                body,
                from_glob: _,
            }) => {
                vis.visit_id(id);
                vis.visit_qself(qself);
                vis.visit_path(path);
                if let Some(rename) = rename {
                    vis.visit_ident(rename);
                }
                if let Some(body) = body {
                    vis.visit_block(body);
                }
            }
        }
    }
}

impl WalkItemKind for AssocItemKind {
    type Ctx = AssocCtxt;
    fn walk(
        &mut self,
        span: Span,
        id: NodeId,
        ident: &mut Ident,
        visibility: &mut Visibility,
        ctxt: Self::Ctxt,
        visitor: &mut impl MutVisitor,
    ) {
        match self {
            AssocItemKind::Const(item) => {
                visit_const_item(item, visitor);
            }
            AssocItemKind::Fn(func) => {
                visitor.visit_fn(
                    FnKind::Fn(FnCtxt::Assoc(ctxt), ident, visibility, &mut *func),
                    span,
                    id,
                );
            }
            AssocItemKind::Type(box TyAlias { defaultness, ty }) => {
                visit_defaultness(visitor, defaultness);
                visit_opt(ty, |ty| visitor.visit_ty(ty));
            }
            AssocItemKind::Delegation(box Delegation {
                id,
                qself,
                path,
                rename,
                body,
                from_glob: _,
            }) => {
                visitor.visit_id(id);
                visitor.visit_qself(qself);
                visitor.visit_path(path);
                if let Some(rename) = rename {
                    visitor.visit_ident(rename);
                }
                if let Some(body) = body {
                    visitor.visit_block(body);
                }
            }
        }
    }
}

fn visit_const_item<T: MutVisitor>(
    ConstItem {
        defaultness,
        generics,
        ty,
        expr,
    }: &mut ConstItem,
    visitor: &mut T,
) {
    visit_defaultness(visitor, defaultness);
    visitor.visit_generics(generics);
    visitor.visit_ty(ty);
    visit_opt(expr, |expr| visitor.visit_expr(expr));
}

fn walk_fn_header<T: MutVisitor>(vis: &mut T, header: &mut FnHeader) {
    let FnHeader { constness } = header;
    visit_constness(vis, constness);
    coroutine_kind
        .as_mut()
        .map(|coroutine_kind| vis.visit_coroutine_kind(coroutine_kind));
    visit_safety(vis, safety);
}

pub fn walk_item(visitor: &mut impl MutVisitor, item: &mut P<Item<impl WalkItemKind<Ctxt = ()>>>) {
    walk_item_ctx(visitor, item, ())
}

pub fn walk_assoc_item(visitor: &mut impl MutVisitor, item: &mut P<AssocItem>, ctxt: AssocCtxt) {
    walk_item_ctx(visitor, item, ctxt)
}

fn walk_item_ctx<K: WalkItemKind>(
    visitor: &mut impl MutVisitor,
    item: &mut P<Item<K>>,
    ctxt: K::Ctxt,
) {
    let Item {
        ident,
        id,
        kind,
        vis,
        span,
        tokens,
    } = item.deref_mut();
    visitor.visit_id(id);
    visit_attrs(visitor, attrs);
    visitor.visit_vis(vis);
    visitor.visit_ident(ident);
    kind.walk(*span, *id, ident, vis, ctxt, visitor);
    visit_lazy_tts(visitor, tokens);
    visitor.visit_span(span);
}

pub fn walk_flat_map_item(vis: &mut impl MutVisitor, mut item: P<Item>) -> SmallVec<[P<Item>; 1]> {
    vis.visit_item(&mut item);
    smallvec![item]
}

pub fn walk_flat_map_foreign_item(
    vis: &mut impl MutVisitor,
    mut item: P<ForeignItem>,
) -> SmallVec<[P<ForeignItem>; 1]> {
    vis.visit_foreign_item(&mut item);
    smallvec![item]
}

pub fn walk_flat_map_assoc_item(
    vis: &mut impl MutVisitor,
    mut item: P<AssocItem>,
    ctxt: AssocCtxt,
) -> SmallVec<[P<AssocItem>; 1]> {
    vis.visit_assoc_item(&mut item, ctxt);
    smallvec![item]
}

impl WalkItemKind for ForeignItemKind {
    type Ctx = ();
    fn walk(
        &mut self,
        span: Span,
        id: NodeId,
        ident: &mut Ident,
        visibility: &mut Visibility,
        _ctxt: Self::Ctxt,
        visitor: &mut impl MutVisitor,
    ) {
        match self {
            ForeignItemKind::Static(box StaticItem {
                ty,
                mutability: _,
                expr,
                safety: _,
            }) => {
                visitor.visit_ty(ty);
                visit_opt(expr, |expr| visitor.visit_expr(expr));
            }
            ForeignItemKind::Fn(func) => {
                visitor.visit_fn(
                    FnKind::Fn(FnCtxt::Foreign, ident, visibility, &mut *func),
                    span,
                    id,
                );
            }
            ForeignItemKind::TyAlias(box TyAlias { defaultness, ty }) => {
                visit_defaultness(visitor, defaultness);
                visit_opt(ty, |ty| visitor.visit_ty(ty));
            }
        }
    }
}

pub fn walk_pat<T: MutVisitor>(vis: &mut T, pat: &mut P<Pattern>) {
    let Pattern {
        id,
        kind,
        span,
        tokens,
    } = pat.deref_mut();
    vis.visit_id(id);
    match kind {
        PatternKind::Err(_guar) => {}
        PatternKind::Wild | PatternKind::Rest | PatternKind::Never => {}
        PatternKind::Ident(_binding_mode, ident, sub) => {
            vis.visit_ident(ident);
            visit_opt(sub, |sub| vis.visit_pat(sub));
        }
        PatternKind::Expr(e) => vis.visit_expr(e),
        PatternKind::TupleStruct(qself, path, elems) => {
            vis.visit_qself(qself);
            vis.visit_path(path);
            visit_thin_vec(elems, |elem| vis.visit_pat(elem));
        }
        PatternKind::Path(qself, path) => {
            vis.visit_qself(qself);
            vis.visit_path(path);
        }
        PatternKind::Struct(qself, path, fields, _etc) => {
            vis.visit_qself(qself);
            vis.visit_path(path);
            fields.flat_map_in_place(|field| vis.flat_map_pat_field(field));
        }
        PatternKind::Box(inner) => vis.visit_pat(inner),
        PatternKind::Deref(inner) => vis.visit_pat(inner),
        PatternKind::Ref(inner, _mutbl) => vis.visit_pat(inner),
        PatternKind::Range(e1, e2, Spanned { span: _, node: _ }) => {
            visit_opt(e1, |e| vis.visit_expr(e));
            visit_opt(e2, |e| vis.visit_expr(e));
            vis.visit_span(span);
        }
        PatternKind::Guard(p, e) => {
            vis.visit_pat(p);
            vis.visit_expr(e);
        }
        PatternKind::Tuple(elems) | PatternKind::Slice(elems) | PatternKind::Or(elems) => {
            visit_thin_vec(elems, |elem| vis.visit_pat(elem))
        }
        PatternKind::Paren(inner) => vis.visit_pat(inner),
        PatternKind::MacCall(mac) => vis.visit_mac_call(mac),
    }
    visit_lazy_tts(vis, tokens);
    vis.visit_span(span);
}

fn walk_anon_const<T: MutVisitor>(vis: &mut T, AnonConst { id, value }: &mut AnonConst) {
    vis.visit_id(id);
    vis.visit_expr(value);
}

fn walk_inline_asm<T: MutVisitor>(vis: &mut T, asm: &mut InlineAsm) {
    // FIXME: Visit spans inside all this currently ignored stuff.
    let InlineAsm {
        asm_macro: _,
        template: _,
        template_strs: _,
        operands,
        clobber_abis: _,
        options: _,
        line_spans: _,
    } = asm;
    for (op, span) in operands {
        match op {
            InlineAsmOperand::In { expr, reg: _ }
            | InlineAsmOperand::Out {
                expr: Some(expr),
                reg: _,
                late: _,
            }
            | InlineAsmOperand::InOut {
                expr,
                reg: _,
                late: _,
            } => vis.visit_expr(expr),
            InlineAsmOperand::Out {
                expr: None,
                reg: _,
                late: _,
            } => {}
            InlineAsmOperand::SplitInOut {
                in_expr,
                out_expr,
                reg: _,
                late: _,
            } => {
                vis.visit_expr(in_expr);
                if let Some(out_expr) = out_expr {
                    vis.visit_expr(out_expr);
                }
            }
            InlineAsmOperand::Const { anon_const } => vis.visit_anon_const(anon_const),
            InlineAsmOperand::Sym { sym } => vis.visit_inline_asm_sym(sym),
            InlineAsmOperand::Label { block } => vis.visit_block(block),
        }
        vis.visit_span(span);
    }
}

fn walk_inline_asm_sym<T: MutVisitor>(
    vis: &mut T,
    InlineAsmSym { id, qself, path }: &mut InlineAsmSym,
) {
    vis.visit_id(id);
    vis.visit_qself(qself);
    vis.visit_path(path);
}

fn walk_format_args<T: MutVisitor>(vis: &mut T, fmt: &mut FormatArgs) {
    // FIXME: visit the template exhaustively.
    let FormatArgs {
        span,
        template: _,
        arguments,
        uncooked_fmt_str: _,
    } = fmt;
    for FormatArgument { kind, expr } in arguments.all_args_mut() {
        match kind {
            FormatArgumentKind::Named(ident) | FormatArgumentKind::Captured(ident) => {
                vis.visit_ident(ident)
            }
            FormatArgumentKind::Normal => {}
        }
        vis.visit_expr(expr);
    }
    vis.visit_span(span);
}

pub fn walk_expr<T: MutVisitor>(
    vis: &mut T,
    Expr {
        kind,
        id,
        span,
        attrs,
        tokens,
    }: &mut Expr,
) {
    vis.visit_id(id);
    visit_attrs(vis, attrs);
    match kind {
        ExprKind::Array(exprs) => visit_thin_exprs(vis, exprs),
        ExprKind::ConstBlock(anon_const) => {
            vis.visit_anon_const(anon_const);
        }
        ExprKind::Repeat(expr, count) => {
            vis.visit_expr(expr);
            vis.visit_anon_const(count);
        }
        ExprKind::Tup(exprs) => visit_thin_exprs(vis, exprs),
        ExprKind::Call(f, args) => {
            vis.visit_expr(f);
            visit_thin_exprs(vis, args);
        }
        ExprKind::MethodCall(box MethodCall {
            seg:
                PathSegment {
                    ident,
                    id,
                    args: seg_args,
                },
            receiver,
            args: call_args,
            span,
        }) => {
            vis.visit_method_receiver_expr(receiver);
            vis.visit_id(id);
            vis.visit_ident(ident);
            visit_opt(seg_args, |args| vis.visit_generic_args(args));
            visit_thin_exprs(vis, call_args);
            vis.visit_span(span);
        }
        ExprKind::Binary(binop, lhs, rhs) => {
            vis.visit_expr(lhs);
            vis.visit_expr(rhs);
            vis.visit_span(&mut binop.span);
        }
        ExprKind::Unary(_unop, ohs) => vis.visit_expr(ohs),
        ExprKind::Cast(expr, ty) => {
            vis.visit_expr(expr);
            vis.visit_ty(ty);
        }
        ExprKind::Type(expr, ty) => {
            vis.visit_expr(expr);
            vis.visit_ty(ty);
        }
        ExprKind::AddrOf(_kind, _mut, ohs) => vis.visit_expr(ohs),
        ExprKind::Let(pat, scrutinee, span, _recovered) => {
            vis.visit_pat(pat);
            vis.visit_expr(scrutinee);
            vis.visit_span(span);
        }
        ExprKind::If(cond, tr, fl) => {
            vis.visit_expr(cond);
            vis.visit_block(tr);
            visit_opt(fl, |fl| ensure_sufficient_stack(|| vis.visit_expr(fl)));
        }
        ExprKind::While(cond, body, label) => {
            visit_opt(label, |label| vis.visit_label(label));
            vis.visit_expr(cond);
            vis.visit_block(body);
        }
        ExprKind::ForLoop {
            pat,
            iter,
            body,
            label,
            kind: _,
        } => {
            visit_opt(label, |label| vis.visit_label(label));
            vis.visit_pat(pat);
            vis.visit_expr(iter);
            vis.visit_block(body);
        }
        ExprKind::Loop(body, label, span) => {
            visit_opt(label, |label| vis.visit_label(label));
            vis.visit_block(body);
            vis.visit_span(span);
        }
        ExprKind::Match(expr, arms, _kind) => {
            vis.visit_expr(expr);
            arms.flat_map_in_place(|arm| vis.flat_map_arm(arm));
        }
        ExprKind::Closure(box Closure {
            binder,
            capture_clause,
            constness,
            coroutine_kind,
            movability: _,
            fn_decl,
            body,
            fn_decl_span,
            fn_arg_span,
        }) => {
            visit_constness(vis, constness);
            vis.visit_capture_by(capture_clause);
            vis.visit_fn(
                FnKind::Closure(binder, coroutine_kind, fn_decl, body),
                *span,
                *id,
            );
            vis.visit_span(fn_decl_span);
            vis.visit_span(fn_arg_span);
        }
        ExprKind::Block(blk, label) => {
            visit_opt(label, |label| vis.visit_label(label));
            vis.visit_block(blk);
        }
        ExprKind::Gen(_capture_by, body, _kind, decl_span) => {
            vis.visit_block(body);
            vis.visit_span(decl_span);
        }
        ExprKind::Await(expr, await_kw_span) => {
            vis.visit_expr(expr);
            vis.visit_span(await_kw_span);
        }
        ExprKind::Use(expr, use_kw_span) => {
            vis.visit_expr(expr);
            vis.visit_span(use_kw_span);
        }
        ExprKind::Assign(el, er, span) => {
            vis.visit_expr(el);
            vis.visit_expr(er);
            vis.visit_span(span);
        }
        ExprKind::AssignOp(_op, el, er) => {
            vis.visit_expr(el);
            vis.visit_expr(er);
        }
        ExprKind::Field(el, ident) => {
            vis.visit_expr(el);
            vis.visit_ident(ident);
        }
        ExprKind::Index(el, er, brackets_span) => {
            vis.visit_expr(el);
            vis.visit_expr(er);
            vis.visit_span(brackets_span);
        }
        ExprKind::Range(e1, e2, _lim) => {
            visit_opt(e1, |e1| vis.visit_expr(e1));
            visit_opt(e2, |e2| vis.visit_expr(e2));
        }
        ExprKind::Underscore => {}
        ExprKind::Path(qself, path) => {
            vis.visit_qself(qself);
            vis.visit_path(path);
        }
        ExprKind::Break(label, expr) => {
            visit_opt(label, |label| vis.visit_label(label));
            visit_opt(expr, |expr| vis.visit_expr(expr));
        }
        ExprKind::Continue(label) => {
            visit_opt(label, |label| vis.visit_label(label));
        }
        ExprKind::Ret(expr) => {
            visit_opt(expr, |expr| vis.visit_expr(expr));
        }
        ExprKind::Yeet(expr) => {
            visit_opt(expr, |expr| vis.visit_expr(expr));
        }
        ExprKind::Become(expr) => vis.visit_expr(expr),
        ExprKind::InlineAsm(asm) => vis.visit_inline_asm(asm),
        ExprKind::FormatArgs(fmt) => vis.visit_format_args(fmt),
        ExprKind::OffsetOf(container, fields) => {
            vis.visit_ty(container);
            for field in fields.iter_mut() {
                vis.visit_ident(field);
            }
        }
        ExprKind::MacCall(mac) => vis.visit_mac_call(mac),
        ExprKind::Struct(se) => {
            let StructExpr {
                qself,
                path,
                fields,
                rest,
            } = se.deref_mut();
            vis.visit_qself(qself);
            vis.visit_path(path);
            fields.flat_map_in_place(|field| vis.flat_map_expr_field(field));
            match rest {
                StructRest::Base(expr) => vis.visit_expr(expr),
                StructRest::Rest(_span) => {}
                StructRest::None => {}
            }
        }
        ExprKind::Paren(expr) => {
            vis.visit_expr(expr);
        }
        ExprKind::Yield(expr) => {
            visit_opt(expr, |expr| vis.visit_expr(expr));
        }
        ExprKind::Try(expr) => vis.visit_expr(expr),
        ExprKind::TryBlock(body) => vis.visit_block(body),
        ExprKind::Lit(_token) => {}
        ExprKind::IncludedBytes(_bytes) => {}
        ExprKind::UnsafeBinderCast(_kind, expr, ty) => {
            vis.visit_expr(expr);
            if let Some(ty) = ty {
                vis.visit_ty(ty);
            }
        }
        ExprKind::Err(_guar) => {}
        ExprKind::Dummy => {}
    }
    visit_lazy_tts(vis, tokens);
    vis.visit_span(span);
}

pub fn noop_filter_map_expr<T: MutVisitor>(vis: &mut T, mut e: P<Expr>) -> Option<P<Expr>> {
    Some({
        vis.visit_expr(&mut e);
        e
    })
}

pub fn walk_flat_map_stmt<T: MutVisitor>(
    vis: &mut T,
    Stmt { kind, span, mut id }: Stmt,
) -> SmallVec<[Stmt; 1]> {
    vis.visit_id(&mut id);
    let mut stmts: SmallVec<[Stmt; 1]> = walk_flat_map_stmt_kind(vis, kind)
        .into_iter()
        .map(|kind| Stmt { id, kind, span })
        .collect();
    match &mut stmts[..] {
        [] => {}
        [stmt] => vis.visit_span(&mut stmt.span),
        _ => panic!(
            "cloning statement `NodeId`s is prohibited by default, \
             the visitor should implement custom statement visiting"
        ),
    }
    stmts
}

fn walk_flat_map_stmt_kind<T: MutVisitor>(vis: &mut T, kind: StmtKind) -> SmallVec<[StmtKind; 1]> {
    match kind {
        StmtKind::Let(mut local) => smallvec![StmtKind::Let({
            vis.visit_local(&mut local);
            local
        })],
        StmtKind::Item(item) => vis
            .flat_map_item(item)
            .into_iter()
            .map(StmtKind::Item)
            .collect(),
        StmtKind::Expr(expr) => vis
            .filter_map_expr(expr)
            .into_iter()
            .map(StmtKind::Expr)
            .collect(),
        StmtKind::Semi(expr) => vis
            .filter_map_expr(expr)
            .into_iter()
            .map(StmtKind::Semi)
            .collect(),
        StmtKind::Empty => smallvec![StmtKind::Empty],
        StmtKind::MacCall(mut mac) => {
            let MacCallStmt {
                mac: mac_,
                style: _,
                attrs,
                tokens,
            } = mac.deref_mut();
            visit_attrs(vis, attrs);
            vis.visit_mac_call(mac_);
            visit_lazy_tts(vis, tokens);
            smallvec![StmtKind::MacCall(mac)]
        }
    }
}

fn walk_vis<T: MutVisitor>(vis: &mut T, visibility: &mut Visibility) {
    let Visibility { kind, span, tokens } = visibility;
    match kind {
        VisibilityKind::Public | VisibilityKind::Inherited => {}
        VisibilityKind::Restricted {
            path,
            id,
            shorthand: _,
        } => {
            vis.visit_id(id);
            vis.visit_path(path);
        }
    }
    visit_lazy_tts(vis, tokens);
    vis.visit_span(span);
}

fn walk_capture_by<T: MutVisitor>(vis: &mut T, capture_by: &mut CaptureBy) {
    match capture_by {
        CaptureBy::Ref => {}
        CaptureBy::Value { move_kw } => {
            vis.visit_span(move_kw);
        }
        CaptureBy::Use { use_kw } => {
            vis.visit_span(use_kw);
        }
    }
}

pub trait DummyAstNode {
    fn dummy() -> Self;
}

impl<T> DummyAstNode for Option<T> {
    fn dummy() -> Self {
        Default::default()
    }
}

impl<T: DummyAstNode + 'static> DummyAstNode for P<T> {
    fn dummy() -> Self {
        P(DummyAstNode::dummy())
    }
}

impl DummyAstNode for Item {
    fn dummy() -> Self {
        Item {
            id: DUMMY_NODE_ID,
            span: Default::default(),
            vis: Visibility {
                kind: VisibilityKind::Public,
                span: Default::default(),
                tokens: Default::default(),
            },
            ident: Ident::dummy(),
            kind: ItemKind::ExternCrate(None),
            tokens: Default::default(),
        }
    }
}

impl DummyAstNode for Expr {
    fn dummy() -> Self {
        Expr {
            id: DUMMY_NODE_ID,
            kind: ExprKind::Dummy,
            span: Default::default(),
            attrs: Default::default(),
            tokens: Default::default(),
        }
    }
}

impl DummyAstNode for Ty {
    fn dummy() -> Self {
        Ty {
            id: DUMMY_NODE_ID,
            kind: TyKind::Dummy,
            span: Default::default(),
            tokens: Default::default(),
        }
    }
}

impl DummyAstNode for Pattern {
    fn dummy() -> Self {
        Pattern {
            id: DUMMY_NODE_ID,
            kind: PatternKind::Wild,
            span: Default::default(),
            tokens: Default::default(),
        }
    }
}

impl DummyAstNode for Stmt {
    fn dummy() -> Self {
        Stmt {
            id: DUMMY_NODE_ID,
            kind: StmtKind::Empty,
            span: Default::default(),
        }
    }
}

impl DummyAstNode for Sapling {
    fn dummy() -> Self {
        Sapling {
            attrs: Default::default(),
            items: Default::default(),
            spans: Default::default(),
            id: DUMMY_NODE_ID,
            is_placeholder: Default::default(),
        }
    }
}

impl<N: DummyAstNode, T: DummyAstNode> DummyAstNode for crate::ast_traits::AstNodeWrapper<N, T> {
    fn dummy() -> Self {
        crate::ast_traits::AstNodeWrapper::new(N::dummy(), T::dummy())
    }
}

#[derive(Debug)]
pub enum FnKind<'a> {
    Fn(FnCtxt, &'a mut Ident, &'a mut Visibility, &'a mut Fn),
    Lambda(
        &'a mut LambdaBinder,
        &'a mut Option<CoroutineKind>,
        &'a mut P<FnDecl>,
        &'a mut P<Expr>,
    ),
}
