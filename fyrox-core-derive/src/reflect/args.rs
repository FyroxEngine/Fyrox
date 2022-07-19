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

impl TypeArgs {
    /// Creates impl [`Generics`] adding bounds
    pub fn impl_generics(&self) -> Generics {
        let mut generics = self.generics.clone();

        // Add where clause for every reflectable field
        let fields: Box<dyn Iterator<Item = &FieldArgs>> = match &self.data {
            ast::Data::Struct(data) => Box::new(data.fields.iter()),
            ast::Data::Enum(variants) => Box::new(variants.iter().flat_map(|v| v.fields.iter())),
        };

        generics.make_where_clause().predicates.extend(
            fields
                .filter(|f| !f.hidden)
                .map(|f| &f.ty)
                .map::<WherePredicate, _>(|ty| parse_quote! { #ty: Reflect }),
        );

        generics
    }
}

#[derive(FromField, Clone, PartialEq)]
#[darling(attributes(reflect))]
pub struct FieldArgs {
    pub ident: Option<Ident>,
    pub ty: Type,

    /// `#[reflect(name = .. )]`
    ///
    /// Property name override for a field (default: snake_case)
    #[darling(default)]
    pub name: Option<String>,

    /// `#[reflect(hidden)]`
    ///
    /// Do not expose the property key
    #[darling(default)]
    pub hidden: bool,
}

#[derive(FromVariant)]
#[darling(attributes(reflect))]
pub struct VariantArgs {
    pub ident: Ident,
    pub fields: ast::Fields<FieldArgs>,
}
