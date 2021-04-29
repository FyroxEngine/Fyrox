use darling::*;
use syn::*;

#[derive(FromDeriveInput)]
#[darling(attributes(visit), supports(struct_any))]
pub struct StructArgs {
    pub ident: Ident,
    // pub vis: Visibility,
    pub generics: Generics,
    pub data: ast::Data<util::Ignored, FieldArgs>,
    // attrs: Vec<Attribute>
}

/// Parsed from struct's or enum variant's field
#[derive(FromField)]
#[darling(attributes(visit))]
pub struct FieldArgs {
    pub ident: Option<Ident>,
    // pub vis: Visibility,
    pub ty: Type,
    // pub attrs: Vec<Attribute>,
    // ---
    /// `#[visit(skip)]`: skip on read and write
    #[darling(default)]
    pub skip: bool,
}
