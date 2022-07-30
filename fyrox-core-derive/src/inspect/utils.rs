// NOTE: The `properties` impl does NOT use Self::PROP_KEY constants, but it's always safe

use convert_case::*;
use darling::ast;
use proc_macro2::TokenStream as TokenStream2;
use quote::*;
use syn::*;

use crate::inspect::args;

/// Handles struct/enum variant field style differences in syntax
#[derive(Clone)]
pub struct FieldPrefix {
    /// Struct | Enum
    is_struct: bool,
    /// Field style of the struct or the enum variant
    style: ast::Style,
    variant: Option<args::VariantArgs>,
}

impl FieldPrefix {
    pub fn of_struct(style: ast::Style) -> Self {
        Self {
            is_struct: true,
            style,
            variant: None,
        }
    }

    pub fn of_enum_variant(args: &args::VariantArgs) -> Self {
        Self {
            is_struct: false,
            style: args.fields.style,
            variant: Some(args.clone()),
        }
    }

    // Returns syntax for binding an enum variant's field on match:
    // ```
    // match x {
    //     X::Struct { a, b, c } => { .. }
    // //             ~~~ use "field_name"
    //     X::Tuple(f0, f1, f2) => { .. }
    // //          ~~~ use "f<index>"
    // }
    // ```
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

    // Returns syntax that corresponds to  `&self.field` for struct, `&variant.field` for structural
    // enum variant, and `&variant.i` for tuple enum variant.
    fn quote_field_ref(
        &self,
        i: usize,
        field: &args::FieldArgs,
        style: ast::Style,
    ) -> TokenStream2 {
        match style {
            ast::Style::Struct => {
                let field_ident = field.ident.as_ref().unwrap();

                if self.is_struct {
                    quote! { (&self.#field_ident) }
                } else {
                    // The `field` is of an enum variant
                    quote!(#field_ident)
                }
            }
            ast::Style::Tuple => {
                let index = Index::from(i);

                if self.is_struct {
                    quote! { (&self.#index) }
                } else {
                    // The `field` is of an enum variant
                    let ident = format_ident!("f{}", index);
                    quote!(#ident)
                }
            }
            ast::Style::Unit => {
                unreachable!()
            }
        }
    }

    // FIXME: Use shared function between `Inspect` and `Reflect`
    fn property_key_name(&self, nth_field: usize, field: &args::FieldArgs) -> String {
        let name = match self.style {
            ast::Style::Struct => {
                format!("{}", field.ident.as_ref().unwrap())
            }
            ast::Style::Tuple => {
                format!("{}", nth_field)
            }
            ast::Style::Unit => {
                unreachable!()
            }
        };

        if let Some(variant) = &self.variant {
            format!("{}@{}", variant.ident, name)
        } else {
            name
        }
    }
}

/// Creates `Inspect` trait impl and field prop keys
pub fn create_inspect_impl<'f>(
    ty_args: &args::TypeArgs,
    field_args: impl Iterator<Item = &'f args::FieldArgs>,
    impl_body: TokenStream2,
) -> TokenStream2 {
    let trait_impl = self::inspect_trait_impl(ty_args, field_args, impl_body);

    quote! {
        #trait_impl
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
    generics.clone()
}

pub fn gen_inspect_fn_body(
    field_prefix: FieldPrefix,
    field_args: &ast::Fields<args::FieldArgs>,
) -> TokenStream2 {
    // `inspect` function body, consisting of a sequence of quotes
    let mut quotes = Vec::new();

    let props = field_args
        .fields
        .iter()
        // enumerate first, and then filter!
        .enumerate()
        .filter(|(_i, f)| !f.skip)
        .map(|(i, field)| self::quote_field_prop(&field_prefix, i, field, field_args.style));

    quotes.push(quote! {
        let mut props = Vec::new();
        #(props.push(#props);)*
    });

    // concatenate the quotes
    quote! {
        #(#quotes)*
        props
    }
}

/// `PropertyInfo { .. }`
fn quote_field_prop(
    field_prefix: &FieldPrefix,
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

    let getter = match &field.getter {
        // use custom getter function to retrieve the target reference
        Some(getter) => {
            quote! { (#field_ref).#getter }
        }
        // default: get reference of the field
        None => field_ref.clone(),
    };

    let prop_key_name = field_prefix.property_key_name(nth_field, field);

    // consider #[inspect(display_name = ..)]
    let display_name = field
        .display_name
        .clone()
        .unwrap_or_else(|| field_ident.to_string());
    let display_name = display_name.to_case(Case::Title);

    let min_value = match field.min_value {
        None => quote! { None },
        Some(v) => quote! { Some(#v)},
    };

    let max_value = match field.max_value {
        None => quote! { None },
        Some(v) => quote! { Some(#v)},
    };

    let step = match field.step {
        None => quote! { None },
        Some(v) => quote! { Some(#v) },
    };

    let precision = match field.precision {
        None => quote! { None },
        Some(v) => quote! { Some(#v) },
    };

    let read_only = field.read_only;

    let description = field.description.clone().unwrap_or_default();

    let is_modified = match field.is_modified.as_ref() {
        Some(getter) => {
            let getter: Path = parse_str(getter).expect("can't parse `is_modified` as a path");
            quote! { #field_ref.#getter() }
        }
        None => quote! { false },
    };

    quote! {
        PropertyInfo {
            owner_type_id: std::any::TypeId::of::<Self>(),
            name: #prop_key_name,
            display_name: #display_name,
            value: #getter,
            read_only: #read_only,
            min_value: #min_value,
            max_value: #max_value,
            step: #step,
            precision: #precision,
            description: (#description).to_string(),
            is_modified: #is_modified,
        }
    }
}
