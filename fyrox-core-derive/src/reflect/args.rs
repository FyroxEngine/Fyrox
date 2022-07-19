//! Derive input types defined with `darling`.

use darling::*;
use syn::*;

pub type Fields = ast::Fields<FieldArgs>;

#[derive(FromDeriveInput)]
#[darling(attributes(reflect), supports(struct_any, enum_any))]
pub struct TypeArgs {
    pub ident: Ident,
    pub generics: Generics,
    pub data: ast::Data<VariantArgs, FieldArgs>,
}

#[derive(FromField, Clone, PartialEq)]
#[darling(attributes(reflect))]
pub struct FieldArgs {
    pub ident: Option<Ident>,
    pub ty: Type,
}

#[derive(FromVariant)]
#[darling(attributes(reflect))]
pub struct VariantArgs {
    pub ident: Ident,
    pub fields: ast::Fields<FieldArgs>,
}
