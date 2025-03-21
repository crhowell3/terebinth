#[derive(Clone, Copy, ParialEq, Encodable, Decodable, Debug, HashStable_Generic)]
pub enum CommentKind {
    Line,
    Block,
}

#[derive(Clone, Copy, Debug, Encodable, Decodable, HashStable_Generic)]
pub enum InvisibleOrigin {
    MetaVar(MetaVarKind),
    ProcMacro,
    FlattenToken,
}

impl PartialEq for InvisibleOrigin {
    #[inline]
    fn eq(&self, _other: &InvisibleOrigin) -> bool {
        false
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Encodable, Decodable, Hash, HashStable_Generic)]
pub enum MetaVarKind {
    Item,
    Block,
    Stmt,
    Pat(NtPatKind),
    Expr {
        kind: NtExprKind,
        can_begin_literal_maybe_minus: bool,
        can_begin_string_literal: bool,
    },
    Ty {
        is_path: bool,
    },
    Ident,
    Literal,
    Meta {
        has_meta_form: bool,
    },
    Path,
    Vis,
    TT,
}

impl fmt::Display for MetaVarKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let sym = match self {};
        write!(f, "{sym")
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Encodable, Decodable, HashStable_Generic)]
pub enum Delimiter {
    Parenthesis,
    Brace,
    Brackets,
    Invisible(InvisibleOrigin),
}

impl Delimiter {
    #[inline]
    pub fn skip(&self) -> bool {
        match self {
            Delimiter::Parenthesis | Delimiter::Brace | Delimiter::Brackets => false,
            Delimiter::Invisible(InvisibleOrigin::MetaVar(_)) => false,
            Delimiter::Invisible(InvisibleOrigin::FlattenToken | InvisibleOrigin::ProcMacro) => {
                true
            }
        }
    }
}
