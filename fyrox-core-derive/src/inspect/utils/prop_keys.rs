//! Generate property keys (constant fields)

use convert_case::{Case, Casing};
use darling::ast;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::*;

use crate::inspect::args;

/// `pub const [VARIANT_]FIELD: &'static str = "key";`
pub fn prop_keys_impl(ty_args: &args::TypeArgs) -> TokenStream2 {
    let ty_ident = &ty_args.ident;
    let (impl_generics, ty_generics, where_clause) = ty_args.generics.split_for_impl();

    let prop_keys = self::quote_prop_keys(ty_args);

    quote! {
        /// Property key constants
        impl #impl_generics #ty_ident #ty_generics #where_clause {
            #prop_keys
        }
    }
}

/// List of `pub const <ident> = <name>;`
///
/// # Definition format
///
/// | Type         | Identifier                | Name                     |
/// |--------------|---------------------------|--------------------------|
/// | Struct       | `FIELD_NAME`              | `field_name`             |
/// | Unit struct  | `F_<number>`              | `<number>`               |
/// | Tuple struct | `VARIANT_NAME_FIELD_NAME  | `VariantName.field_name` |
/// | Enum         | `VARIANT_NAME_F_<number>` | `VariantName.<number>`   |
pub fn quote_prop_keys(ty_args: &args::TypeArgs) -> TokenStream2 {
    let mut prop_idents = Vec::new();
    let mut prop_key_names = Vec::new();

    match &ty_args.data {
        ast::Data::Struct(field_args) => {
            for (nth, field) in field_args.fields.iter().enumerate() {
                if field.skip {
                    continue;
                }

                let prop_ident = self::struct_prop_ident(ty_args, nth, field);
                let prop_key_name = self::struct_prop_key_name(nth, field);

                prop_idents.push(prop_ident);
                prop_key_names.push(prop_key_name);
            }
        }
        ast::Data::Enum(variants) => {
            for v in variants {
                for (nth, field) in v.fields.iter().enumerate() {
                    if field.skip {
                        continue;
                    }

                    let prop_ident = self::enum_prop_ident(v, nth, field);
                    let prop_key_name = self::enum_prop_key_name(nth, field, v);

                    prop_idents.push(prop_ident);
                    prop_key_names.push(prop_key_name);
                }
            }
        }
    }

    quote! {
        #(
            #[allow(missing_docs)]
            pub const #prop_idents: &'static str = #prop_key_names;
        )*
    }
}

pub fn struct_prop_key_name(nth: usize, field: &args::FieldArgs) -> String {
    field.name.clone().unwrap_or_else(|| {
        let field_ident = match &field.ident {
            Some(ident) => quote!(#ident),
            None => {
                let nth_field = Index::from(nth);
                quote!(#nth_field)
            }
        };

        field_ident.to_string()
    })
}

pub fn enum_prop_key_name(nth: usize, field: &args::FieldArgs, v: &args::VariantArgs) -> String {
    field.name.clone().unwrap_or_else(|| {
        let field_ident = match &field.ident {
            Some(ident) => quote!(#ident),
            None => {
                let nth_field = Index::from(nth);
                quote!(#nth_field)
            }
        };

        format!("{}.{}", v.ident, field_ident)
    })
}

pub fn struct_prop_ident(ty_args: &args::TypeArgs, nth: usize, field: &args::FieldArgs) -> Ident {
    let fields = match &ty_args.data {
        ast::Data::Struct(xs) => xs,
        _ => unreachable!(),
    };
    let field_ident = self::field_ident(fields, nth, field);

    let ident = field_ident.to_case(Case::UpperSnake);
    syn::parse_str(&ident).unwrap()
}

pub fn enum_prop_ident(
    variant_args: &args::VariantArgs,
    nth: usize,
    field: &args::FieldArgs,
) -> Ident {
    let variant_ident = &variant_args.ident;
    let field_ident = self::field_ident(&variant_args.fields, nth, field);

    let ident = format!("{}_{}", variant_ident, field_ident).to_case(Case::UpperSnake);
    syn::parse_str(&ident).unwrap()
}

fn field_ident(fields: &args::Fields, nth: usize, field: &args::FieldArgs) -> String {
    match fields.style {
        ast::Style::Struct => field.ident.as_ref().unwrap().to_string(),
        ast::Style::Tuple => {
            // this is actually `F_0` in UPPER_SNAKE_CASE
            format!("F{}", nth)
        }
        ast::Style::Unit => unreachable!(),
    }
}
