mod diagnostic;
mod diagnostic_builder;
mod error;
mod subdiagnostic;
mod utils;

use diagnostic::{DiagnosticDerive, LintDiagnosticDerive};
use proc_macro2::TokenStream;
use subdiagnostic::SubdiagnosticDerive;
use synstructure::Structure;

pub(super) fn diagnostic_derive(mut s: Structure<'_>) -> TokenStream {
    s.underscore_const(true);
    DiagnosticDerive::new(s).into_tokens()
}

pub(super) fn lint_diagnostic_derive(mut s: Structure<'_>) -> TokenStream {
    s.underscore_const(true);
    LintDiagnosticDerive::new(s).into_tokens()
}

pub(super) fn subdiagnostic_derive(mut s: Structure<'_>) -> TokenStream {
    s.underscore_const(true);
    SubdiagnosticDerive::new().into_tokens(s)
}
