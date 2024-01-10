use darling::*;
use syn::*;

#[derive(FromDeriveInput)]
#[darling(attributes(visit), supports(struct_any, enum_any))]
pub struct TypeArgs {
    pub ident: Ident,
    pub generics: Generics,
    pub data: ast::Data<VariantArgs, FieldArgs>,
    #[darling(default)]
    pub optional: bool,
}

/// Parsed from struct's or enum variant's field
#[derive(FromField, Clone)]
#[darling(attributes(visit))]
pub struct FieldArgs {
    pub ident: Option<Ident>,
    // pub vis: Visibility,
    pub ty: Type,
    // pub attrs: Vec<Attribute>,
    // ---
    /// `#[visit(skip)]`
    ///
    /// Skip on read and write
    #[darling(default)]
    pub skip: bool,

    /// `#[visit(rename = "..")]`
    ///
    /// Force this name
    #[darling(default)]
    pub rename: Option<String>,

    /// `#[visit(optional)]`
    ///
    /// Ignore missing field
    #[darling(default)]
    pub optional: bool,
}

#[derive(FromVariant)]
#[darling(attributes(inspect))]
pub struct VariantArgs {
    pub ident: Ident,
    pub fields: ast::Fields<FieldArgs>,
}
