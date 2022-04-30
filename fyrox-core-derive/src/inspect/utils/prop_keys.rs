//! Generate property keys (constant fields)

use convert_case::{Case, Casing};
use darling::ast;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::*;

use crate::inspect::{args, utils};

/// Returns a list of `pub const [VARIANT_]FIELD: &'static str = "key_value"`;
pub fn quote_prop_keys(ty_args: &args::TypeArgs) -> TokenStream2 {
    let mut prop_idents = Vec::new();
    let mut prop_names = Vec::new();

    match &ty_args.data {
        ast::Data::Struct(field_args) => {
            for (nth, field) in field_args.fields.iter().enumerate() {
                if field.skip {
                    continue;
                }

                let prop_ident = self::struct_field_prop(ty_args, nth, field);
                let prop_name = utils::prop_name(nth, field);

                prop_idents.push(prop_ident);
                prop_names.push(prop_name);
            }
        }
        ast::Data::Enum(variants) => {
            for v in variants {
                for (nth, field) in v.fields.iter().enumerate() {
                    if field.skip {
                        continue;
                    }

                    let prop_ident = self::enum_field_prop(v, nth, field);
                    let prop_name = utils::prop_name(nth, field);

                    prop_idents.push(prop_ident);
                    prop_names.push(prop_name);
                }
            }
        }
    }

    quote! {
        #(
            #[allow(missing_docs)]
            pub const #prop_idents: &'static str = #prop_names;
        )*
    }
}

fn struct_field_prop(ty_args: &args::TypeArgs, nth: usize, field: &args::FieldArgs) -> Ident {
    let fields = match &ty_args.data {
        ast::Data::Struct(xs) => xs,
        _ => unreachable!(),
    };
    let field_ident = self::field_ident(fields, nth, field);

    let ident = field_ident.to_case(Case::UpperSnake);
    syn::parse_str(&ident).unwrap()
}

fn enum_field_prop(variant_args: &args::VariantArgs, nth: usize, field: &args::FieldArgs) -> Ident {
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
