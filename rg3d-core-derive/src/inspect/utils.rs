use darling::ast;
use proc_macro2::TokenStream as TokenStream2;
use quote::*;
use syn::*;

use convert_case::*;

use crate::inspect::args;

/// Creates `impl Inspect` block for struct type
pub fn create_impl(
    ty_args: &args::TypeArgs,
    field_args: impl Iterator<Item = args::FieldArgs>,
    impl_body: TokenStream2,
) -> TokenStream2 {
    let ty_ident = &ty_args.ident;
    let generics = self::create_impl_generics(&ty_args.generics, field_args);
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    quote! {
        impl #impl_generics Inspect for #ty_ident #ty_generics #where_clause {
            fn properties(&self) -> Vec<PropertyInfo<'_>> {
                #impl_body
            }
        }
    }
}

/// Creates where clause for `impl Inspect` block
fn create_impl_generics(
    generics: &Generics,
    _field_args: impl Iterator<Item = args::FieldArgs>,
) -> Generics {
    let generics = generics.clone();

    // add boundaries if nessesary

    generics
}

pub fn create_field_properties<'a>(
    // <self.> or <variant.>
    prefix: TokenStream2,
    fields: impl Iterator<Item = &'a args::FieldArgs>,
    field_style: ast::Style,
) -> Vec<TokenStream2> {
    assert_eq!(
        field_style,
        ast::Style::Struct,
        "#[derive(Inspect)] handles only named fields for now"
    );

    let mut bodies = vec![];

    for field in fields {
        // we know it is named field
        let field_ident = field.ident.as_ref().unwrap();

        let field_name = field_ident.to_string().to_case(Case::UpperSnake);
        let group = "Common";
        let value = quote! { #prefix #field_ident };

        let body = quote! {
            PropertyInfo {
                name: #field_name,
                group: #group,
                value: &#value,
            }
        };

        bodies.push(body);
    }

    bodies
}
