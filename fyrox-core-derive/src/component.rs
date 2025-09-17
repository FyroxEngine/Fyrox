// Copyright (c) 2019-present Dmitry Stepanov and Fyrox Engine contributors.
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

use darling::*;
use proc_macro2::{TokenStream as TokenStream2, TokenStream};
use quote::*;
use syn::*;

#[derive(FromDeriveInput)]
#[darling(attributes(component), supports(struct_any, enum_any))]
pub struct TypeArgs {
    pub ident: Ident,
    pub generics: Generics,
    pub data: ast::Data<VariantArgs, FieldArgs>,
}

#[derive(FromField, Clone)]
#[darling(attributes(component))]
pub struct FieldArgs {
    pub ident: Option<Ident>,
    pub ty: Type,
    #[darling(default)]
    pub include: bool,
    #[darling(default)]
    pub path: Option<String>,
    #[darling(default)]
    pub dest_type: Option<Type>,
}

#[derive(FromVariant)]
#[darling(attributes(component))]
#[allow(dead_code)] // TODO: Add support for enum variants.
pub struct VariantArgs {
    pub ident: Ident,
    pub fields: ast::Fields<FieldArgs>,
}

pub fn impl_type_uuid_provider(ast: DeriveInput) -> TokenStream2 {
    let ty_args = TypeArgs::from_derive_input(&ast).unwrap();
    match &ty_args.data {
        ast::Data::Struct(ref field_args) => impl_type_uuid_provider_struct(&ty_args, field_args),
        ast::Data::Enum(_) => unimplemented!(),
    }
}

fn create_field_components(
    // false if enum variant
    is_struct: bool,
    fields: &[FieldArgs],
    field_style: ast::Style,
    mutable: bool,
) -> Vec<TokenStream2> {
    if field_style == ast::Style::Unit {
        // `Unit` struct/enum variant has no components.
        return Default::default();
    }

    let prefix = if is_struct {
        if mutable {
            quote!(&mut self.)
        } else {
            quote!(&self.)
        }
    } else {
        // For enum variant, the match should be handled elsewhere.
        quote!(/* enum variant not handled here */)
    };

    fields
        .iter()
        .filter(|arg| arg.include)
        .map(|arg| {
            let ident = &arg.ident;
            let ty = &arg.ty;

            let field_expr = if let Some(path) = &arg.path {
                let path: TokenStream = parse_str(path).expect("Invalid path");
                quote!(#prefix #path)
            } else {
                quote!(#prefix #ident)
            };

            let target_type = if let Some(dest_type) = &arg.dest_type {
                quote!(#dest_type)
            } else {
                quote!(#ty)
            };

            quote! {
                if type_id == std::any::TypeId::of::<#target_type>() {
                    return Ok(#field_expr as _);
                } else {
                    visited_types.push(std::any::TypeId::of::<#target_type>());
                }
            }
        })
        .collect()
}

fn impl_type_uuid_provider_struct(
    ty_args: &TypeArgs,
    field_args: &ast::Fields<FieldArgs>,
) -> TokenStream2 {
    let ty_ident = &ty_args.ident;
    let (impl_generics, ty_generics, where_clause) = ty_args.generics.split_for_impl();

    let component_ref_checks = if field_args.style != ast::Style::Unit {
        let fields = &field_args.fields;
        fields
            .iter()
            .map(|f| {
                let ident = &f.ident;
                let ty = &f.ty;

                quote! {
                    if type_id == std::any::TypeId::of::<#ty>() {
                        return Ok(&self.#ident as &dyn std::any::Any);
                    } else {
                        visited_types.push(std::any::TypeId::of::<#ty>());
                    }
                }
            })
            .collect()
    } else {
        vec![]
    };

    let component_mut_checks = if field_args.style != ast::Style::Unit {
        let fields = &field_args.fields;
        fields
            .iter()
            .map(|f| {
                let ident = &f.ident;
                let ty = &f.ty;

                quote! {
                    if type_id == std::any::TypeId::of::<#ty>() {
                        return Ok(&mut self.#ident as &mut dyn std::any::Any);
                    } else {
                        visited_types.push(std::any::TypeId::of::<#ty>());
                    }
                }
            })
            .collect()
    } else {
        vec![]
    };

    quote! {
        impl #impl_generics ComponentProvider for #ty_ident #ty_generics #where_clause {
            fn query_component_ref(
                &self,
                type_id: std::any::TypeId,
            ) -> Result<&dyn std::any::Any, QueryComponentError> {
                if type_id == std::any::TypeId::of::<Self>() {
                    return Ok(self);
                }

                let mut visited_types = Vec::new();

                #(#component_ref_checks)*

                Err(QueryComponentError::new(
                    type_id,
                    std::any::TypeId::of::<Self>(),
                    visited_types
                ))
            }

            fn query_component_mut(
                &mut self,
                type_id: std::any::TypeId,
            ) -> Result<&mut dyn std::any::Any, QueryComponentError> {
                if type_id == std::any::TypeId::of::<Self>() {
                    return Ok(self);
                }

                let mut visited_types = Vec::new();

                #(#component_mut_checks)*

                Err(QueryComponentError::new(
                    type_id,
                    std::any::TypeId::of::<Self>(),
                    visited_types
                ))
            }
        }
    }
}
