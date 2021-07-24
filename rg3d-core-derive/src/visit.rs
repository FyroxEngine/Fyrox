mod args;

use std::collections::HashSet;

use convert_case::{Case, Casing};
use darling::*;
use proc_macro2::TokenStream as TokenStream2;
use quote::*;
use syn::*;

// impl `#[derive(Visit)]` for `struct` or `enum`
pub fn impl_visit(ast: DeriveInput) -> TokenStream2 {
    let args = args::TypeArgs::from_derive_input(&ast).unwrap();
    match &args.data {
        ast::Data::Struct(ref field_args) => self::impl_visit_struct(&args, field_args),
        ast::Data::Enum(ref variants) => self::impl_visit_enum(&args, variants),
    }
}

fn create_generics(
    generics: &Generics,
    field_args: impl Iterator<Item = args::FieldArgs>,
) -> Generics {
    let mut generics = generics.clone();

    // Add where clause for every visited field
    generics.make_where_clause().predicates.extend(
        field_args
            .filter(|f| !f.skip)
            .map(|f| f.ty)
            .map::<WherePredicate, _>(|ty| parse_quote! { #ty: Visit }),
    );

    generics
}

/// `<prefix>field.visit("name", visitor);`
fn create_field_visits<'a>(
    // None or `f` when bindings tuple variants. NOTE: We can't use `prefix: Ident`
    prefix: Option<Ident>,
    fields: impl Iterator<Item = &'a args::FieldArgs>,
    field_style: ast::Style,
) -> Vec<TokenStream2> {
    if field_style == ast::Style::Unit {
        // `Unit` (struct/enum variant) has no field to visit.
        // We won't even enter this region:
        return vec![];
    }

    let visit_args = fields
        .filter(|field| !field.skip)
        .enumerate()
        .map(|(field_index, field)| {
            let (ident, name) = match field_style {
                // `NamedFields { a: f32, .. }`
                ast::Style::Struct => {
                    let ident = field.ident.as_ref().unwrap_or_else(|| unreachable!());

                    (
                        quote!(#ident),
                        format!("{}", ident).to_case(Case::UpperCamel),
                    )
                }
                // `Tuple(f32, ..)`
                ast::Style::Tuple => {
                    let ident = Index::from(field_index);

                    let ident = match prefix {
                        Some(ref prefix) => {
                            let ident = format_ident!("{}{}", prefix, ident);
                            quote!(#ident)
                        }
                        None => quote!(#ident),
                    };

                    (ident, format!("{}", field_index))
                }
                ast::Style::Unit => unreachable!(),
            };

            let name = match &field.rename {
                Some(new_name) => {
                    assert!(
                        !new_name.is_empty(),
                        "renaming to empty string doesn't make sense!"
                    );
                    // overwrite the field name with the specified name:
                    new_name.clone()
                }
                None => name,
            };

            (ident, name, field.optional)
        })
        .collect::<Vec<_>>();

    let mut no_dup = HashSet::new();
    for name in visit_args.iter().map(|(_, name, _)| name) {
        if !no_dup.insert(name) {
            panic!("duplicate visiting names detected!");
        }
    }

    visit_args
        .iter()
        .map(|(ident, name, optional)| {
            if *optional {
                quote! {
                    #ident.visit(#name, visitor).ok();
                }
            } else {
                quote! {
                    #ident.visit(#name, visitor)?;
                }
            }
        })
        .collect::<Vec<_>>()
}

/// impl `Visit` for `struct`
fn impl_visit_struct(
    args: &args::TypeArgs,
    field_args: &ast::Fields<args::FieldArgs>,
) -> TokenStream2 {
    let ty_ident = &args.ident;
    let generics = self::create_generics(&args.generics, field_args.iter().cloned());
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let visit_fn_body = if field_args.style == ast::Style::Unit {
        quote! { Ok(()) }
    } else {
        // `field.visit(..);` parts
        let field_visits =
            self::create_field_visits(None, field_args.fields.iter(), field_args.style);

        quote! {
            visitor.enter_region(name)?;
            #(self.#field_visits)*
            visitor.leave_region()
        }
    };

    quote! {
        impl #impl_generics Visit for #ty_ident #ty_generics #where_clause {
            fn visit(
                &mut self,
                #[allow(unused)]
                name: &str,
                #[allow(unused)]
                visitor: &mut Visitor,
            ) -> VisitResult {
                #visit_fn_body
            }
        }
    }
}

/// impl `Visit` for `enum`
fn impl_visit_enum(args: &args::TypeArgs, variants: &[args::VariantArgs]) -> TokenStream2 {
    let ty_ident = &args.ident;
    let ty_name = format!("{}", ty_ident);

    // variant ID = variant index
    let id_type = quote!(u32);

    // `fn id(&self) -> u32`
    let fn_id = {
        let matchers = variants.iter().enumerate().map(|(variant_index, variant)| {
            let variant_index = variant_index as u32;
            let variant_ident = &variant.ident;

            match variant.fields.style {
                ast::Style::Struct => quote! {
                    #ty_ident::#variant_ident { .. } => #variant_index,
                },
                ast::Style::Tuple => {
                    let idents = (0..variant.fields.len()).map(|__| quote!(_));

                    quote! {
                        #ty_ident::#variant_ident(#(#idents),*) => #variant_index,
                    }
                }
                ast::Style::Unit => quote! {
                    #ty_ident::#variant_ident => #variant_index,
                },
            }
        });

        let (impl_generics, ty_generics, where_clause) = args.generics.split_for_impl();

        quote! {
            fn id #impl_generics (me: &#ty_ident #ty_generics) -> #id_type #where_clause {
                match me {
                    #(#matchers)*
                    _ => unreachable!("Unable to get ID from enum variant") ,
                }
            }
        }
    };

    // `fn from_id(id: u32) -> Result<Self, String>`
    let fn_from_id = {
        // `<variant_index> => Ok(TypeName::Variant(Default::default())),
        let matchers = variants.iter().enumerate().map(|(variant_index, variant)| {
            let variant_index = variant_index as u32;
            let variant_ident = &variant.ident;

            // create default value of this variant
            let default = match variant.fields.style {
                ast::Style::Struct => {
                    let defaults = variant.fields.iter().map(|field| {
                        let field_ident = &field.ident;
                        quote! {
                            #field_ident: Default::default(),
                        }
                    });

                    quote! {
                        #ty_ident::#variant_ident {
                            #(#defaults)*
                        },
                    }
                }
                ast::Style::Tuple => {
                    let defaults = variant
                        .fields
                        .iter()
                        .map(|_| quote! { Default::default(), });

                    quote! {
                        #ty_ident::#variant_ident(#(#defaults)*),
                    }
                }
                ast::Style::Unit => quote! {
                    #ty_ident::#variant_ident
                },
            };

            quote! {
                id if id == #variant_index => Ok(#default),
            }
        });

        let (impl_generics, ty_generics, where_clause) = args.generics.split_for_impl();

        quote! {
            fn from_id #impl_generics (
                id: #id_type
            ) -> std::result::Result<#ty_ident #ty_generics, String>
                #where_clause
            {
                match id {
                    #(#matchers)*
                    _ => Err(format!("Unknown ID for type `{}`: `{}`", #ty_name, id)),
                }
            }
        }
    };

    // visit every field of each variant
    let variant_visits = variants.iter().map(|variant| {
        let (fields, style) = (&variant.fields, variant.fields.style);
        let variant_ident = &variant.ident;

        match style {
            ast::Style::Struct => {
                let field_visits = self::create_field_visits(None, fields.iter(), style);

                let idents = fields.iter().map(|field| {
                    let ident = &field.ident;
                    quote!(#ident)
                });

                quote! {
                    #ty_ident::#variant_ident { #(#idents),* } => {
                        #(#field_visits)*
                    },
                }
            }
            ast::Style::Tuple => {
                let field_visits = self::create_field_visits(parse_quote!(f), fields.iter(), style);

                let idents = (0..fields.len()).map(|i| format_ident!("f{}", Index::from(i)));

                quote! {
                    #ty_ident::#variant_ident(#(#idents),*) => {
                        #(#field_visits)*
                    },
                }
            }
            ast::Style::Unit => quote! {
                #ty_ident::#variant_ident => {},
            },
        }
    });

    let generics = {
        // fields of all the variants
        let field_args = variants
            .iter()
            .flat_map(|variant| variant.fields.clone().into_iter());

        self::create_generics(&args.generics, field_args)
    };

    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    // plain-enum-only support
    quote! {
        impl #impl_generics Visit for #ty_ident #ty_generics #where_clause {
            fn visit(
                &mut self,
                name: &str,
                visitor: &mut Visitor,
            ) -> VisitResult {
                visitor.enter_region(name)?;

                let mut id = id(self);
                id.visit("Id", visitor)?;

                if visitor.is_reading() {
                    *self = from_id(id)?;
                }

                match self {
                    #(#variant_visits)*
                }

                return visitor.leave_region();

                #fn_id

                #fn_from_id
            }
        }
    }
}
