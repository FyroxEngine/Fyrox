// NOTE: The `properties` impl does NOT use Self::PROP_KEY constants, but it's always safe

mod prop_keys;

use darling::ast;
use proc_macro2::TokenStream as TokenStream2;
use quote::*;
use syn::*;

use convert_case::*;

use crate::inspect::args;

/// Handles struct/enum field style combinations
///
/// Struct fields are referred to as `self.<field>`, while enum variants are decomposed in `match` arm.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FieldPrefix {
    /// Struct | Enum
    is_struct: bool,
    /// Field style of the struct or the enum variant
    style: ast::Style,
}

impl FieldPrefix {
    pub fn of_struct(style: ast::Style) -> Self {
        Self {
            is_struct: true,
            style,
        }
    }

    pub fn of_enum_variant(style: ast::Style) -> Self {
        Self {
            is_struct: false,
            style,
        }
    }

    pub fn field_match_ident(self, i: usize, field: &args::FieldArgs, style: ast::Style) -> Ident {
        assert!(!self.is_struct);

        match style {
            ast::Style::Struct => field.ident.clone().unwrap(),
            ast::Style::Tuple => {
                let i = Index::from(i);
                format_ident!("f{}", i)
            }
            ast::Style::Unit => {
                unreachable!()
            }
        }
    }

    fn quote_field_ref(self, i: usize, field: &args::FieldArgs, style: ast::Style) -> TokenStream2 {
        match style {
            ast::Style::Struct => {
                let field_ident = field.ident.as_ref().unwrap();

                if self.is_struct {
                    quote! { (&self.#field_ident) }
                } else {
                    quote!(#field_ident)
                }
            }
            ast::Style::Tuple => {
                let index = Index::from(i);

                if self.is_struct {
                    quote! { (&self.#index) }
                } else {
                    let ident = format_ident!("f{}", index);
                    quote!(#ident)
                }
            }
            ast::Style::Unit => {
                unreachable!()
            }
        }
    }
}

/// Creates `Inspect` trait impl and field prop keys
pub fn create_inspect_impl<'f>(
    ty_args: &args::TypeArgs,
    field_args: impl Iterator<Item = &'f args::FieldArgs>,
    impl_body: TokenStream2,
) -> TokenStream2 {
    let prop_keys_impl = self::prop_keys_impl(ty_args);
    let trait_impl = self::inspect_trait_impl(ty_args, field_args, impl_body);

    quote! {
        #prop_keys_impl
        #trait_impl
    }
}

/// `pub const [VARIANT_]FIELD: &'static str = "key";`
fn prop_keys_impl(ty_args: &args::TypeArgs) -> TokenStream2 {
    let ty_ident = &ty_args.ident;
    let (impl_generics, ty_generics, where_clause) = ty_args.generics.split_for_impl();

    let prop_keys = prop_keys::quote_prop_keys(ty_args);
    quote! {
        /// Property key constants
        impl #impl_generics #ty_ident #ty_generics #where_clause {
            #prop_keys
        }
    }
}

/// `impl Inspect`
fn inspect_trait_impl<'f>(
    ty_args: &args::TypeArgs,
    field_args: impl Iterator<Item = &'f args::FieldArgs>,
    impl_body: TokenStream2,
) -> TokenStream2 {
    let ty_ident = &ty_args.ident;
    let generics = self::impl_inspect_generics(&ty_args.generics, field_args);
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
///
/// TODO: Add `where Field: Inspect` boundaries to support inspectable types with generics
fn impl_inspect_generics<'a>(
    generics: &Generics,
    _field_args: impl Iterator<Item = &'a args::FieldArgs>,
) -> Generics {
    let generics = generics.clone();

    generics
}

pub fn gen_inspect_fn_body(
    ty_args: &args::TypeArgs,
    field_prefix: FieldPrefix,
    field_args: &ast::Fields<args::FieldArgs>,
) -> TokenStream2 {
    let owner_name = ty_args.ident.to_string();

    // `inspect` function body, consisting of a sequence of quotes
    let mut quotes = Vec::new();

    // 1. collect non-expanible field properties
    let props = field_args
        .fields
        .iter()
        .enumerate()
        .filter(|(_i, f)| !f.skip && !(f.expand || f.expand_subtree))
        .map(|(i, field)| {
            self::quote_field_prop(&owner_name, field_prefix, i, field, field_args.style)
        });

    quotes.push(quote! {
        let mut props = Vec::new();
        #(props.push(#props);)*
    });

    // 2. visit expansible fields
    for (i, field) in field_args
        .fields
        .iter()
        .enumerate()
        .filter(|(_i, f)| !f.skip && (f.expand || f.expand_subtree))
    {
        // parent (the field)
        if field.expand_subtree {
            let prop =
                self::quote_field_prop(&owner_name, field_prefix, i, field, field_args.style);

            quotes.push(quote! {
                props.push(#prop);
            });
        }

        // children (fields of the field)
        let field_ref = field_prefix.quote_field_ref(i, field, field_args.style);

        quotes.push(quote! {
            props.extend(#field_ref.properties());
        });
    }

    // concatanate the quotes
    quote! {
        #(#quotes)*
        props
    }
}

/// `PropertyInfo { .. }`
fn quote_field_prop(
    // the name of the property owner, used as default property group
    owner_name: &str,
    field_prefix: FieldPrefix,
    nth_field: usize,
    field: &args::FieldArgs,
    style: ast::Style,
) -> TokenStream2 {
    let field_ident = match &field.ident {
        Some(ident) => quote!(#ident),
        None => {
            let nth_field = Index::from(nth_field);
            quote!(#nth_field)
        }
    };

    let field_ref = field_prefix.quote_field_ref(nth_field, field, style);

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

    let read_only = field.read_only;

    quote! {
        PropertyInfo {
            owner_type_id: std::any::TypeId::of::<Self>(),
            name: #field_name,
            display_name: #display_name,
            group: #group,
            value: #field_ref,
            read_only: #read_only,
        }
    }
}

pub fn prop_name(nth: usize, field: &args::FieldArgs) -> String {
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
