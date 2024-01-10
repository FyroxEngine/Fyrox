mod args;
mod utils;

use darling::*;
use proc_macro2::TokenStream as TokenStream2;
use quote::*;
use syn::*;

// impl `#[derive(Visit)]` for `struct` or `enum`
pub fn impl_visit(ast: DeriveInput) -> TokenStream2 {
    let ty_args = args::TypeArgs::from_derive_input(&ast).unwrap();
    match &ty_args.data {
        ast::Data::Struct(ref field_args) => self::impl_visit_struct(&ty_args, field_args),
        ast::Data::Enum(ref variants) => self::impl_visit_enum(&ty_args, variants),
    }
}

/// impl `Visit` for `struct`
fn impl_visit_struct(
    ty_args: &args::TypeArgs,
    field_args: &ast::Fields<args::FieldArgs>,
) -> TokenStream2 {
    let visit_fn_body = if field_args.style == ast::Style::Unit {
        quote! { Ok(()) }
    } else {
        // `field.visit(..)?;` parts
        let field_visits = utils::create_field_visits(
            true,
            ty_args.optional,
            field_args.fields.iter(),
            field_args.style,
        );

        quote! {
            let mut region = match visitor.enter_region(name) {
                Ok(x) => x,
                Err(err) => return Err(err),
            };
            #(#field_visits)*
            Ok(())
        }
    };

    utils::create_impl(ty_args, field_args.iter().cloned(), visit_fn_body)
}

/// impl `Visit` for `enum`
fn impl_visit_enum(ty_args: &args::TypeArgs, variant_args: &[args::VariantArgs]) -> TokenStream2 {
    let ty_ident = &ty_args.ident;
    let ty_name = format!("{}", ty_ident);

    // variant ID = variant index
    let id_type = quote!(u32);

    // `fn id(&self) -> u32`
    let fn_id = {
        let matchers = variant_args
            .iter()
            .enumerate()
            .map(|(variant_index, variant)| {
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

        let (impl_generics, ty_generics, where_clause) = ty_args.generics.split_for_impl();

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
        let matchers = variant_args
            .iter()
            .enumerate()
            .map(|(variant_index, variant)| {
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

        let (impl_generics, ty_generics, where_clause) = ty_args.generics.split_for_impl();

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
    let variant_visits = variant_args.iter().map(|variant| {
        let (fields, style) = (&variant.fields, variant.fields.style);
        let variant_ident = &variant.ident;

        match style {
            ast::Style::Struct => {
                let field_visits =
                    utils::create_field_visits(false, ty_args.optional, fields.iter(), style);

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
                let field_visits =
                    utils::create_field_visits(false, ty_args.optional, fields.iter(), style);

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

    utils::create_impl(
        ty_args,
        variant_args.iter().flat_map(|v| v.fields.iter()).cloned(),
        quote! {
             let mut region = match visitor.enter_region(name) {
                 Ok(x) => x,
                 Err(err) => return Err(err),
             };

             let mut id = id(self);
             if let Err(err) = id.visit("Id", &mut region) {
                 return Err(err);
             };

             if region.is_reading() {
                 *self = match from_id(id) {
                     Ok(x) => x,
                     Err(s) => return Err(s.into()),
                 };
             }

             match self {
                 #(#variant_visits)*
             }

             return Ok(());

             #fn_id

             #fn_from_id
        },
    )
}
