//! Implements `Inspect` trait

mod args;
mod utils;

use darling::{ast, FromDeriveInput};
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use syn::*;

pub fn impl_inspect(ast: DeriveInput) -> TokenStream2 {
    let ty_args = args::TypeArgs::from_derive_input(&ast).unwrap();
    match &ty_args.data {
        ast::Data::Struct(ref field_args) => self::impl_inspect_struct(&ty_args, field_args),
        ast::Data::Enum(ref variant_args) => self::impl_inspect_enum(&ty_args, variant_args),
    }
}

fn impl_inspect_struct(ty_args: &args::TypeArgs, field_args: &args::Fields) -> TokenStream2 {
    let body = utils::gen_inspect_fn_body(ty_args, utils::FieldPrefix::Self_, field_args);
    utils::create_impl(ty_args, field_args.iter(), body)
}

fn impl_inspect_enum(ty_args: &args::TypeArgs, variant_args: &[args::VariantArgs]) -> TokenStream2 {
    let variant_matches =
        variant_args.iter().map(|variant| {
            let variant_ident = &variant.ident;

            let field_prefix = if variant.fields.style == ast::Style::Struct {
                utils::FieldPrefix::None
            } else {
                utils::FieldPrefix::F
            };

            let field_idents = variant.fields.fields.iter().enumerate().map(|(i, field)| {
                match variant.fields.style {
                    ast::Style::Struct => field.ident.clone().unwrap(),
                    ast::Style::Tuple => {
                        let i = Index::from(i);
                        format_ident!("f{}", i)
                    }
                    ast::Style::Unit => {
                        format_ident!("<unreachable>")
                    }
                }
            });

            let variant_match = match variant.fields.style {
                ast::Style::Struct => {
                    quote! {
                        Self::#variant_ident { #(#field_idents),* }
                    }
                }
                ast::Style::Tuple => {
                    quote! {
                        Self::#variant_ident(#(#field_idents),*)
                    }
                }
                ast::Style::Unit => {
                    quote! {
                        Self::#variant_ident
                    }
                }
            };

            let fields_inspect = utils::gen_inspect_fn_body(ty_args, field_prefix, &variant.fields);

            quote! {
                #variant_match => {
                    #fields_inspect
                }
            }
        });

    let body = quote! {
        match self {
            #(#variant_matches,)*
        }
    };

    let field_args = variant_args.iter().flat_map(|v| v.fields.iter());
    utils::create_impl(ty_args, field_args, body)
}
