use darling::*;
use syn::*;

#[derive(FromDeriveInput)]
#[darling(supports(struct_any))]
pub struct StructArgs {
    pub ident: Ident,
    // pub vis: Visibility,
    pub generics: Generics,
    pub data: ast::Data<(), FieldArgs>,
    // attrs: Vec<Attribute>
}

#[derive(FromField)]
pub struct FieldArgs {
    pub ident: Option<Ident>,
    // pub vis: Visibility,
    pub ty: Type,
    // pub attrs: Vec<Attribute>,
}
