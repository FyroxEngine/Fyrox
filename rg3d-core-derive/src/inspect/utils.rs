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

pub fn gen_inspect_fn_body(
    ty_args: &args::TypeArgs,
    field_prefix: TokenStream2,
    field_args: &ast::Fields<args::FieldArgs>,
) -> TokenStream2 {
    let owner_name = ty_args.ident.to_string();

    // `inspect` function body, consisting of a sequence of quotes
    let mut quotes = Vec::new();

    // collect non-expanible field properties
    let props = field_args
        .fields
        .iter()
        .filter(|f| !f.skip && !(f.expand || f.expand_subtree))
        .map(|field| self::quote_field_prop(&field_prefix, &owner_name, field));

    quotes.push(quote! {
        let mut props = Vec::new();
        #(props.push(#props);)*
    });

    // visit expanible fields
    for field in field_args
        .fields
        .iter()
        .filter(|f| !f.skip && (f.expand || f.expand_subtree))
    {
        // parent (the field)
        if field.expand_subtree {
            let prop = self::quote_field_prop(&field_prefix, &owner_name, field);
            quotes.push(quote! {
                props.push(#prop);
            });
        }

        // children (fields of the field)
        let field_ident = field.ident.as_ref().expect("named field expected");
        quotes.push(quote! {
            props.extend(#field_prefix #field_ident .properties());
        });
    }

    // concatanate the quotes
    quote! {
        #(#quotes)*
        props
    }
}

fn quote_field_prop(
    // `self.`, none or `f`
    field_prefix: &TokenStream2,
    // the name of the property owner, used as default property group
    owner_name: &str,
    field: &args::FieldArgs,
) -> TokenStream2 {
    // we know it is a named field
    let field_ident = field.ident.as_ref().expect("named field expected");

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
        .unwrap_or(owner_name);

    let value = quote! { #field_prefix #field_ident };

    quote! {
        PropertyInfo {
            owner_type_id: std::any::TypeId::of::<Self>(),
            name: #field_name,
            display_name: #display_name,
            group: #group,
            value: &#value,
        }
    }
}
