//! Generate property keys (constant fields)

use convert_case::{Case, Casing};
use darling::ast;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::*;

use crate::reflect::args;

/// Property key constant
///
/// # Formats
///
/// | Type           | Identifier                | Name                     |
/// |----------------|---------------------------|--------------------------|
/// | Struct         | `FIELD_NAME`              | `field_name`             |
/// | Struct (tuple) | `F_<number>`              | `<number>`               |
/// | Enum (struct)  | `VARIANT_NAME_FIELD_NAME  | `VariantName@field_name` |
/// | Enum (tuple)   | `VARIANT_NAME_F_<number>` | `VariantName@<number>`   |
pub struct Property<'a> {
    /// Property constant identifier
    pub ident: Ident,
    /// Property constant value
    pub value: String,
    /// Identifier or the index of the field the property refers to
    pub field_quote: TokenStream2,
    /// Original field
    pub field: &'a args::FieldArgs,
}

impl<'a> Property<'a> {
    pub fn quote(&self) -> TokenStream2 {
        let Property { ident, value, .. } = self;

        quote! {
            pub const #ident: &'static str = #value;
        }
    }
}

pub fn impl_prop_constants<'a, 'b: 'a>(
    props: impl Iterator<Item = &'a Property<'b>>,
    ty_ident: &Ident,
    generics: &Generics,
) -> TokenStream2 {
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    let props = props.map(|p| p.quote());

    quote! {
        #[allow(missing_docs)]
        impl #impl_generics #ty_ident #ty_generics #where_clause {
            #( #props )*
        }
    }
}

fn field_ident(nth: usize, field: &args::FieldArgs) -> TokenStream2 {
    match &field.ident {
        Some(ident) => quote!(#ident),
        None => {
            let nth_field = Index::from(nth);
            quote!(#nth_field)
        }
    }
}

pub fn enum_prop<'a>(
    variant: &args::VariantArgs,
    nth: usize,
    field: &'a args::FieldArgs,
) -> Property<'a> {
    let ident = self::enum_prop_ident(variant, nth, field);
    let value = self::enum_prop_value(variant, nth, field);

    Property {
        ident,
        value,
        field_quote: self::field_ident(nth, field),
        field,
    }
}

pub fn struct_prop<'a>(
    ty_args: &args::TypeArgs,
    nth: usize,
    field: &'a args::FieldArgs,
) -> Property<'a> {
    let ident = self::struct_prop_ident(ty_args, nth, field);
    let value = self::struct_prop_value(nth, field);

    Property {
        ident,
        value,
        field_quote: self::field_ident(nth, field),
        field,
    }
}

pub fn props(ty_args: &args::TypeArgs) -> Box<dyn Iterator<Item = Property<'_>> + '_> {
    match &ty_args.data {
        ast::Data::Struct(field_args) => Box::new(
            field_args
                .fields
                .iter()
                .enumerate()
                .filter(|(_, f)| !f.hidden)
                .map(|(nth, field)| self::struct_prop(ty_args, nth, field)),
        ),
        ast::Data::Enum(variants) => Box::new(variants.iter().flat_map(|v| {
            v.fields
                .iter()
                .enumerate()
                .filter(|(_, f)| !f.hidden)
                .map(|(nth, field)| self::enum_prop(v, nth, field))
        })),
    }
}

// --------------------------------------------------------------------------------
// Identifiers
// --------------------------------------------------------------------------------

/// Struct (`FIELD_NAME`) | Tuple (`F0`)
pub fn struct_prop_ident(ty_args: &args::TypeArgs, nth: usize, field: &args::FieldArgs) -> Ident {
    let fields = match &ty_args.data {
        ast::Data::Struct(xs) => xs,
        _ => unreachable!(),
    };
    let field_ident = self::field_ident_string(fields, nth, field);

    let ident = field_ident.to_case(Case::UpperSnake);
    syn::parse_str(&ident).unwrap()
}

/// Struct (`EnumVariant_FIELD_NAME`) | Tuple (`EnumVariant_F_0`)
pub fn enum_prop_ident(
    variant_args: &args::VariantArgs,
    nth: usize,
    field: &args::FieldArgs,
) -> Ident {
    let variant_ident = &variant_args.ident;
    let field_ident = self::field_ident_string(&variant_args.fields, nth, field);

    let ident = format!("{}_{}", variant_ident, field_ident).to_case(Case::UpperSnake);
    syn::parse_str(&ident).unwrap()
}

fn field_ident_string(fields: &args::Fields, nth: usize, field: &args::FieldArgs) -> String {
    match fields.style {
        ast::Style::Struct => field.ident.as_ref().unwrap().to_string(),
        ast::Style::Tuple => {
            // this is actually `F_0` in UPPER_SNAKE_CASE
            format!("F{}", nth)
        }
        ast::Style::Unit => unreachable!(),
    }
}

// --------------------------------------------------------------------------------
// Values
// --------------------------------------------------------------------------------

/// Struct (`"field_name"`) | Tuple (`"0"`)
pub fn struct_prop_value(nth: usize, field: &args::FieldArgs) -> String {
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

/// Struct (`"EnumVariant.filed_name"`) | Tuple (`"EnumVariant.0"`)
pub fn enum_prop_value(v: &args::VariantArgs, nth: usize, field: &args::FieldArgs) -> String {
    field.name.clone().unwrap_or_else(|| {
        let field_ident = match &field.ident {
            Some(ident) => quote!(#ident),
            None => {
                let nth_field = Index::from(nth);
                quote!(#nth_field)
            }
        };

        format!("{}@{}", v.ident, field_ident)
    })
}
