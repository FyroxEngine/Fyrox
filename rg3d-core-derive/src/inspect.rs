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
        "#[derive(Inspect)] handles only named fields for now"
    );

    let body = utils::gen_inspect_fn_body(ty_args, quote! { self. }, field_args);
    utils::create_impl(ty_args, field_args.iter().cloned(), body)
}

fn impl_inspect_enum(
    _ty_args: &args::TypeArgs,
    _variant_args: &[args::VariantArgs],
) -> TokenStream2 {
    todo!("#[derive(Inspect)] is only for structure types for now")
    // bind each field

    // generate implementation
}
