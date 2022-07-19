//! Implements `Reflect` trait

mod args;
mod prop;
mod syntax;

use darling::{ast, FromDeriveInput};
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::*;

pub fn impl_reflect(ast: DeriveInput) -> TokenStream2 {
    let ty_args = args::TypeArgs::from_derive_input(&ast).unwrap();
    match &ty_args.data {
        ast::Data::Struct(ref field_args) => self::impl_reflect_struct(&ty_args, field_args),
        ast::Data::Enum(ref variant_args) => self::impl_reflect_enum(&ty_args, variant_args),
    }
}

fn impl_reflect_struct(ty_args: &args::TypeArgs, _field_args: &args::Fields) -> TokenStream2 {
    let prop_keys = prop::props(ty_args);
    let prop_idents = prop_keys.iter().map(|p| &p.ident).collect::<Vec<_>>();
    let field_idents = prop_keys.iter().map(|p| &p.field_ident).collect::<Vec<_>>();

    let field = quote! {
        match name {
            #(
                Self::#prop_idents => Some(&self.#field_idents),
            )*
            _ => None,
        }
    };

    let field_mut = quote! {
        match name {
            #(
                Self::#prop_idents => Some(&mut self.#field_idents),
            )*
            _ => None,
        }
    };

    self::gen_impl(ty_args, field, field_mut)
}

fn impl_reflect_enum(ty_args: &args::TypeArgs, variant_args: &[args::VariantArgs]) -> TokenStream2 {
    let getters = variant_args
        .iter()
        .map(|v| {
            let fields = v
                .fields
                .iter()
                .enumerate()
                .filter(|(_, f)| !f.hidden)
                .collect::<Vec<_>>();

            let props = fields.iter().map(|(i, f)| {
                let prop = prop::enum_prop(v, *i, f);
                prop.ident
            });

            let syntax = syntax::VariantSyntax::new(ty_args.ident.clone(), v);
            let matcher = syntax.matcher();

            let field_idents = fields.iter().map(|(i, f)| {
                let field_ident = syntax.field_match_ident(*i, f);

                quote! {
                    #field_ident
                }
            });

            quote! {
                #(
                    Self::#props => match self {
                        #matcher => #field_idents,
                        _ => return None,
                    },
                )*
            }
        })
        .collect::<Vec<_>>();

    let field = quote! {
        Some(match name {
            #(
                #getters
            )*
            _ => return None,
        })
    };

    let field_mut = quote! {
        Some(match name {
            #(
                #getters
            )*
            _ => return None,
        })
    };

    self::gen_impl(ty_args, field, field_mut)
}

fn gen_impl(
    ty_args: &args::TypeArgs,
    field: TokenStream2,
    field_mut: TokenStream2,
) -> TokenStream2 {
    let ty_ident = &ty_args.ident;
    let generics = ty_args.impl_generics();
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let prop_keys = prop::props(ty_args);
    let prop_key_impl = prop::impl_prop_keys(prop_keys.iter(), &ty_args.ident, &generics);

    quote! {
        #prop_key_impl

        #[allow(warnings)]
        impl #impl_generics Reflect for #ty_ident #ty_generics #where_clause {
            fn into_any(self: Box<Self>) -> Box<dyn ::core::any::Any> {
                self
            }

            fn set(&mut self, value: Box<dyn Reflect>) -> Result<(), Box<dyn Reflect>> {
                *self = value.take()?;
                Ok(())
            }

            fn as_any(&self) -> &dyn ::core::any::Any {
                self
            }

            fn as_any_mut(&mut self) -> &mut dyn ::core::any::Any {
                self
            }

            fn as_reflect(&self) -> &dyn Reflect {
                self
            }

            fn as_reflect_mut(&mut self) -> &mut dyn Reflect {
                self
            }

            fn field(&self, name: &str) -> Option<&dyn Reflect> {
                #field
            }

            fn field_mut(&mut self, name: &str) -> Option<&mut dyn Reflect> {
                #field_mut
            }
        }
    }
}
