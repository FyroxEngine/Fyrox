//! Implements `Reflect` trait

mod args;

use darling::{ast, FromDeriveInput};
use proc_macro2::TokenStream as TokenStream2;
use syn::*;

pub fn impl_reflect(ast: DeriveInput) -> TokenStream2 {
    let ty_args = args::TypeArgs::from_derive_input(&ast).unwrap();
    match &ty_args.data {
        ast::Data::Struct(ref field_args) => self::impl_reflect_struct(&ty_args, field_args),
        ast::Data::Enum(ref variant_args) => self::impl_reflect_enum(&ty_args, variant_args),
    }
}

fn impl_reflect_struct(ty_args: &args::TypeArgs, field_args: &args::Fields) -> TokenStream2 {
    todo!()
}

fn impl_reflect_enum(ty_args: &args::TypeArgs, variant_args: &[args::VariantArgs]) -> TokenStream2 {
    todo!()
}
