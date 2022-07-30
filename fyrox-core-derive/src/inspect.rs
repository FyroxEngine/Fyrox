//! Implements `Inspect` trait

mod args;
mod utils;

use darling::{ast, FromDeriveInput};
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::*;

pub fn impl_inspect(ast: DeriveInput) -> TokenStream2 {
    let mut ty_args = args::TypeArgs::from_derive_input(&ast).unwrap();
    ty_args.validate();

    match &ty_args.data {
        ast::Data::Struct(ref field_args) => self::impl_inspect_struct(&ty_args, field_args),
        ast::Data::Enum(ref variant_args) => self::impl_inspect_enum(&ty_args, variant_args),
    }
}

fn impl_inspect_struct(ty_args: &args::TypeArgs, field_args: &args::Fields) -> TokenStream2 {
    let field_prefix = utils::FieldPrefix::of_struct(field_args.style);
    let body = utils::gen_inspect_fn_body(field_prefix, field_args);
    utils::create_inspect_impl(ty_args, field_args.iter(), body)
}

fn impl_inspect_enum(ty_args: &args::TypeArgs, variant_args: &[args::VariantArgs]) -> TokenStream2 {
    let variant_matches = variant_args.iter().map(|variant| {
        let variant_ident = &variant.ident;

        let field_prefix = utils::FieldPrefix::of_enum_variant(variant);

        let field_match_idents = variant.fields.fields.iter().enumerate().map(|(i, field)| {
            let field_prefix = field_prefix.clone();
            field_prefix.field_match_ident(i, field, variant.fields.style)
        });

        let variant_match = match variant.fields.style {
            ast::Style::Struct => {
                quote! {
                    Self::#variant_ident { #(#field_match_idents),* }
                }
            }
            ast::Style::Tuple => {
                quote! {
                    Self::#variant_ident(#(#field_match_idents),*)
                }
            }
            ast::Style::Unit => {
                quote! {
                    Self::#variant_ident
                }
            }
        };

        let fields_inspects = utils::gen_inspect_fn_body(field_prefix, &variant.fields);

        quote! {
            #variant_match => {
                #fields_inspects
            }
        }
    });

    let body = quote! {
        match self {
            #(#variant_matches,)*
        }
    };

    let field_args = variant_args.iter().flat_map(|v| v.fields.iter());
    utils::create_inspect_impl(ty_args, field_args, body)
}
