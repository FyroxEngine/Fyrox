//! Implements `Inspect` trait

mod args;
mod utils;

use darling::{ast, FromDeriveInput};
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::*;

pub fn impl_inspect(ast: DeriveInput) -> TokenStream2 {
    let ty_args = args::TypeArgs::from_derive_input(&ast).unwrap();
    match &ty_args.data {
        ast::Data::Struct(ref field_args) => self::impl_inspect_struct(&ty_args, field_args),
        ast::Data::Enum(ref variant_args) => self::impl_inspect_enum(&ty_args, variant_args),
    }
}

fn impl_inspect_struct(
    ty_args: &args::TypeArgs,
    field_args: &ast::Fields<args::FieldArgs>,
) -> TokenStream2 {
    assert_eq!(
        field_args.style,
        ast::Style::Struct,
        "#[derive(Inspect) considers only named fields for now"
    );

    let prop_vec = {
        let props = utils::collect_field_props(
            quote! {  self. },
            field_args.fields.iter(),
            field_args.style,
        );

        quote! {
            vec![
                #(
                    #props,
                )*
            ]
        }
    };

    // list of `self.expanded_field.prop()`
    let prop_calls = utils::collect_field_prop_calls(
        quote! {  self. },
        field_args.fields.iter(),
        field_args.style,
    );

    let impl_body = if prop_calls.is_empty() {
        prop_vec
    } else {
        // NOTE: All items marked as `#[inspect(expand)]` come to the end of the property list
        quote! {
            let mut props = #prop_vec;
            #(
                props.extend(#prop_calls .into_iter());
            )*
            props
        }
    };

    utils::create_impl(ty_args, field_args.iter().cloned(), impl_body)
}

fn impl_inspect_enum(
    _ty_args: &args::TypeArgs,
    _variant_args: &[args::VariantArgs],
) -> TokenStream2 {
    todo!("#[derive(Inspect)] is only for structure types for now")
}
