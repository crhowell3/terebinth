use proc_macro::TokenStream;
use synstructure::decl_derive;

mod current_version;
mod serialize;

#[proc_macro]
pub fn current_terebinth_version(input: TokenStream) -> TokenStream {
    current_version::current_version(input)
}

#[proc_macro]
pub fn terebinth_queries(input: TokenStream) -> TokenStream {
    query::terebinth_queries(input)
}

#[proc_macro]
pub fn symbols(input: TokenStream) -> TokenStream {
    symbols::symbols(input.into()).into()
}

#[proc_macro_attribute]
pub fn extension(attr: TokenStream, input: TokenStream) -> TokenStream {
    extension::extension(attr, input)
}

decl_derive!([HashStable, attributes(stable_hasher)] => hash_stable::hash_stable_derive);
decl_derive!([HashStable_Generic, attributes(stable_hasher)] => hash_stable::hash_stable_generic_derive);
decl_derive!([HashStable_NoContext] => hash_stable::hash_stable_no_context_derive);
decl_derive!([Decodable_NoContext] => serialize::decodable_nocontext_derive);
decl_derive!([Encodable_NoContext] => serialize::encodable_nocontext_derive);
decl_derive!([Decodable] => serialize::decodable_derive);
decl_derive!([Encodable] => serialize::encodable_derive);
decl_derive!([TyDecodable] => serialize::type_decodable_derive);
decl_derive!([TyEncodable] => serialize::type_encodable_derive);
decl_derive!([MetadataDecodable] => serialize::meta_decodable_derive);
decl_derive!([MetadataEncodable] => serialize::meta_encodable_derive);
decl_derive!(
    [TypeFoldable, attributes(type_foldable)] =>
    type_foldable::type_foldable_derive
);
decl_derive!(
    [TypeVisitable, attributes(type_visitable)] =>
    type_visitable::type_visitable_derive
);
decl_derive!([Lift, attributes(lift)] => lift::lift_derive);
decl_derive!(
    [Diagnostic, attributes(
        diag,
        help,
        help_once,
        note,
        note_once,
        warning,
        skip_arg,
        primary_span,
        label,
        subdiagnostic,
        suggestion,
        suggestion_short,
        suggestion_hidden,
        suggestion_verbose)] => diagnostics::diagnostic_derive
);
decl_derive!(
    [LintDiagnostic, attributes(
        diag,
        help,
        help_once,
        note,
        note_once,
        warning,
        skip_arg,
        primary_span,
        label,
        subdiagnostic,
        suggestion,
        suggestion_short,
        suggestion_hidden,
        suggestion_verbose)] => diagnostics::lint_diagnostic_derive
);
decl_derive!(
    [Subdiagnostic, attributes(
        label,
        help,
        help_once,
        note,
        note_once,
        warning,
        subdiagnostic,
        suggestion,
        suggestion_short,
        suggestion_hidden,
        suggestion_verbose,
        multipart_suggestion,
        multipart_suggestion_short,
        multipart_suggestion_hidden,
        multipart_suggestion_verbose,
        skip_arg,
        primary_span,
        suggestion_part,
        applicability)] => diagnostics::subdiagnostic_derive
);

decl_derive! {
    [TryFromU32] =>
    try_from::try_from_u32
}
decl_derive! {
    [PrintAttribute] =>
    print_attribute::print_attribute
}
