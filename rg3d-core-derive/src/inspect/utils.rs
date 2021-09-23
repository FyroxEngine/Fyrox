use darling::ast;
use proc_macro2::TokenStream as TokenStream2;
use quote::*;
use syn::*;

use convert_case::*;

use crate::inspect::args;

/// Creates `impl Inspect` block
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

/// Creates `Generic` for `impl Inspect` block
fn create_impl_generics(
    generics: &Generics,
    _field_args: impl Iterator<Item = args::FieldArgs>,
) -> Generics {
    let generics = generics.clone();

    // add boundaries if nessesary

    generics
}

/// List of `PropertyInfo { .. }`
pub fn collect_field_props<'a>(
    // <self.> or <variant.>
    prefix: TokenStream2,
    owner_name: String,
    fields: impl Iterator<Item = &'a args::FieldArgs>,
    field_style: ast::Style,
) -> Vec<TokenStream2> {
    assert_eq!(
        field_style,
        ast::Style::Struct,
        "#[derive(Inspect)] handles only named fields for now"
    );

    let mut bodies = Vec::new();

    // consider #[inspect(skip)]
    for field in fields.filter(|f| !f.skip && (!f.expand || f.include_self)) {
        // we know it is a named field
        let field_ident = field.ident.as_ref().unwrap();

        // consider #[inspect(name = ..)]
        let field_name = field
            .name
            .clone()
            .unwrap_or_else(|| field_ident.to_string());

        // consider #[inspect(display_name = ..)]
        let display_name = field
            .display_name
            .clone()
            .unwrap_or_else(|| field_ident.to_string());
        let display_name = display_name.to_case(Case::Title);

        // consider #[inspect(group = ..)]
        let group = field
            .group
            .as_ref()
            .map(|s| s.as_str())
            .unwrap_or(owner_name.as_str());

        let value = quote! { #prefix #field_ident };

        let body = quote! {
            PropertyInfo {
                owner_type_id: std::any::TypeId::of::<Self>(),
                name: #field_name,
                display_name: #display_name,
                group: #group,
                value: &#value,
            }
        };

        bodies.push(body);
    }

    bodies
}

/// List of `field.properties()` for each `#[inspect(expand)]` field
pub fn collect_field_prop_calls<'a>(
    // `self.` or `variant.`
    prefix: TokenStream2,
    fields: impl Iterator<Item = &'a args::FieldArgs>,
    field_style: ast::Style,
) -> Vec<TokenStream2> {
    assert_eq!(
        field_style,
        ast::Style::Struct,
        "#[derive(Inspect)] handles only named fields for now"
    );

    let mut expands = Vec::new();

    for field in fields.filter(|f| !f.skip && f.expand) {
        // we know it is a named field
        let field_ident = field.ident.as_ref().unwrap();

        expands.push(quote! {
            #prefix #field_ident .properties()
        });
    }

    expands
}
