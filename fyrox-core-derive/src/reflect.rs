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

//! Implements `Reflect` trait

pub mod args;
mod prop;
mod syntax;

use convert_case::{Case, Casing};
use darling::ast;
use proc_macro2::TokenStream as TokenStream2;
use prop::Property;
use quote::quote;
use syn::Index;

pub fn impl_reflect(ty_args: &args::TypeArgs) -> TokenStream2 {
    if ty_args.hide_all {
        return self::gen_impl(ty_args, None, quote!(func(&[])), quote!(func(&mut [])));
    }

    match &ty_args.data {
        ast::Data::Struct(ref field_args) => self::impl_reflect_struct(ty_args, field_args),
        ast::Data::Enum(ref variant_args) => self::impl_reflect_enum(ty_args, variant_args),
    }
}

pub fn impl_prop_constants(ty_args: &args::TypeArgs) -> TokenStream2 {
    let prop_keys = prop::props(ty_args).collect::<Vec<_>>();
    prop::impl_prop_constants(prop_keys.iter(), &ty_args.ident, &ty_args.generics)
}

pub fn gen_fields_metadata_body(
    props: &[Property],
    field_getters: &[TokenStream2],
    field_args: &ast::Fields<args::FieldArgs>,
    is_mut: bool,
) -> TokenStream2 {
    let props = field_args
        .fields
        .iter()
        .enumerate()
        .filter(|(_i, f)| !f.hidden)
        .zip(props.iter().zip(field_getters))
        .map(|((i, field), (prop, field_getter))| {
            self::quote_field_prop(&prop.value, i, field_getter, field, is_mut)
        });

    quote! {
        #(#props,)*
    }
}

/// `FieldInfo { .. }`
fn quote_field_prop(
    prop_key_name: &str,
    nth_field: usize,
    field_getter: &TokenStream2,
    field: &args::FieldArgs,
    is_mut: bool,
) -> TokenStream2 {
    let field_ident = match &field.ident {
        Some(ident) => quote!(#ident),
        None => {
            let nth_field = Index::from(nth_field);
            quote!(#nth_field)
        }
    };

    let doc = args::fetch_doc_comment(&field.attrs);

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

    let tag = field.tag.clone().unwrap_or_default();

    let read_only = field.read_only;

    let immutable_collection = field.immutable_collection;

    let description = field.description.clone().unwrap_or_default();

    let variant = if is_mut {
        quote! { FieldMut }
    } else {
        quote! { FieldRef }
    };

    quote! {
        {
            static METADATA: FieldMetadata = FieldMetadata {
                name: #prop_key_name,
                display_name: #display_name,
                tag: #tag,
                doc: #doc,
                read_only: #read_only,
                immutable_collection: #immutable_collection,
                min_value: #min_value,
                max_value: #max_value,
                step: #step,
                precision: #precision,
                description: #description,
            };

            #variant {
                metadata: &METADATA,
                value: #field_getter,
            }
        }
    }
}

fn impl_reflect_struct(ty_args: &args::TypeArgs, field_args: &args::Fields) -> TokenStream2 {
    // Property keys for `Reflect::{field, field_mut, set_field}` impls:
    let props = prop::props(ty_args).collect::<Vec<_>>();

    let (fields, field_muts): (Vec<_>, Vec<_>) = props
        .iter()
        .map(|p| {
            let quote = &p.field_quote;
            (quote!(&self.#quote), quote!(&mut self.#quote))
        })
        .unzip();

    let (fields, field_muts) = self::collect_field_refs(&props, &fields, &field_muts);
    let fields = fields.collect::<Vec<_>>();
    let field_muts = field_muts.collect::<Vec<_>>();

    let metadata_ref = gen_fields_metadata_body(&props, &fields, field_args, false);
    let metadata_mut = gen_fields_metadata_body(&props, &field_muts, field_args, true);

    let set_field_body = self::struct_set_field_body(ty_args);
    self::gen_impl(
        ty_args,
        set_field_body,
        quote! {
            func(&[#metadata_ref])
        },
        quote! {
            func(&mut [#metadata_mut])
        },
    )
}

fn struct_set_field_body(ty_args: &args::TypeArgs) -> Option<TokenStream2> {
    let props = prop::props(ty_args)
        .filter(|p| p.field.setter.is_some())
        .collect::<Vec<_>>();

    if props.is_empty() {
        return None;
    }

    let prop_values = props.iter().map(|p| &p.value);

    let set_fields = props.iter().map(|p| {
        let setter = p.field.setter.as_ref().unwrap();
        quote! {{
            func(match value.take() {
                Ok(value) => {
                    let prev = self.#setter(value);
                    Ok(Box::new(prev))
                }
                Err(current) => {
                    Err(current)
                }
            })
        }}
    });

    Some(quote! {
        match name {
            #(
                #prop_values => #set_fields,
            )*
            _ => {
                let mut opt_value = Some(value);
                self.field_mut(name, &mut move |field| {
                    let value = opt_value.take().unwrap();
                    match field {
                        Some(f) => func(f.set(value)),
                        None => func(Err(value)),
                    };
                });
            },
        }
    })
}

fn impl_reflect_enum(ty_args: &args::TypeArgs, variant_args: &[args::VariantArgs]) -> TokenStream2 {
    let mut fields_ref_ref = Vec::new();
    let mut fields_mut = Vec::new();
    for v in variant_args.iter() {
        let fields = v
            .fields
            .iter()
            .enumerate()
            .filter(|(_, f)| !f.hidden)
            .collect::<Vec<_>>();

        let props = fields
            .iter()
            .map(|(i, f)| prop::enum_prop(v, *i, f))
            .collect::<Vec<_>>();

        let syntax = syntax::VariantSyntax::new(ty_args.ident.clone(), v);
        let matcher = syntax.matcher();

        let (fields, field_muts): (Vec<_>, Vec<_>) = fields
            .iter()
            .map(|(i, f)| {
                let field_quote = syntax.field_match_ident(*i, f);
                (quote!(#field_quote), quote!(#field_quote))
            })
            .unzip();

        let (fields, field_muts) = self::collect_field_refs(&props, &fields, &field_muts);
        let fields = fields.collect::<Vec<_>>();
        let field_muts = field_muts.collect::<Vec<_>>();

        let metadata_ref = gen_fields_metadata_body(&props, &fields, &v.fields, false);
        let metadata_mut = gen_fields_metadata_body(&props, &field_muts, &v.fields, true);

        fields_ref_ref.push(quote! {
            #matcher => func(&[#metadata_ref]),
        });

        fields_mut.push(quote! {
            #matcher => func(&mut [#metadata_mut]),
        });
    }

    let fields_metadata_ref_body = quote! {
        match self {
            #(
                #fields_ref_ref
            )*
            _ => func(&[])
        }
    };

    let fields_metadata_mut_body = quote! {
        match self {
            #(
                #fields_mut
            )*
            _ => func(&mut [])
        }
    };

    self::gen_impl(
        ty_args,
        None,
        fields_metadata_ref_body,
        fields_metadata_mut_body,
    )
}

#[allow(clippy::too_many_arguments)]
fn gen_impl(
    ty_args: &args::TypeArgs,
    set_field: Option<TokenStream2>,
    metadata_ref: TokenStream2,
    metadata_mut: TokenStream2,
) -> TokenStream2 {
    let ty_ident = &ty_args.ident;
    let generics = ty_args.impl_generics();
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let as_list_impl = ty_args.as_list_impl();
    let as_array_impl = ty_args.as_array_impl();

    let doc = args::fetch_doc_comment(&ty_args.attrs);
    let assembly_name = std::env::var("CARGO_PKG_NAME").unwrap_or_default();

    let set_field = set_field.map(|set_field| {
        quote! {
            fn set_field(&mut self, name: &str, value: Box<dyn Reflect>, func: &mut dyn FnMut(Result<Box<dyn Reflect>, Box<dyn Reflect>>),) {
                #set_field
            }
        }
    });

    let types = ty_args
        .derived_type
        .iter()
        .map(|ty| {
            quote! { std::any::TypeId::of::<#ty>() }
        })
        .collect::<Vec<TokenStream2>>();
    let types = quote! { #(#types),* };

    quote! {
        #[allow(warnings)]
        impl #impl_generics Reflect for #ty_ident #ty_generics #where_clause {
            fn source_path() -> &'static str {
                file!()
            }

            fn type_name(&self) -> &'static str {
                std::any::type_name::<Self>()
            }

             fn derived_types() -> &'static [std::any::TypeId] {
                static ARRAY: std::sync::LazyLock<Vec<std::any::TypeId>> = std::sync::LazyLock::new(|| vec![
                    #types
                ]);

                &ARRAY
            }

            fn query_derived_types(&self) -> &'static [std::any::TypeId] {
                Self::derived_types()
            }

            fn doc(&self) -> &'static str {
                #doc
            }

            fn assembly_name(&self) -> &'static str {
                #assembly_name
            }

            fn type_assembly_name() -> &'static str {
                #assembly_name
            }

            fn fields_ref(&self, func: &mut dyn FnMut(&[FieldRef])) {
                #metadata_ref
            }

            fn fields_mut(&mut self, func: &mut dyn FnMut(&mut [FieldMut])) {
                #metadata_mut
            }

            fn into_any(self: Box<Self>) -> Box<dyn ::core::any::Any> {
                self
            }

            fn set(&mut self, value: Box<dyn Reflect>) -> Result<Box<dyn Reflect>, Box<dyn Reflect>> {
                let value = match value.take() {
                    Ok(x) => x,
                    Err(err) => return Err(err),
                };
                let this = std::mem::replace(self, value);
                Ok(Box::new(this))
            }

            #set_field

            fn as_any(&self, func: &mut dyn FnMut(&dyn ::core::any::Any)) {
                func(self)
            }

            fn as_any_mut(&mut self, func: &mut dyn FnMut(&mut dyn ::core::any::Any)) {
                func(self)
            }

            fn as_reflect(&self, func: &mut dyn FnMut(&dyn Reflect)) {
                func(self as &dyn Reflect)
            }

            fn as_reflect_mut(&mut self, func: &mut dyn FnMut(&mut dyn Reflect)) {
                func(self as &mut dyn Reflect)
            }

            #as_array_impl

            #as_list_impl
        }
    }
}

/// Collects field references for match RHS, excluding `#[reflect(setter = ..)]` fields
fn collect_field_refs<'a, 'b: 'a>(
    props: &'b [Property<'a>],
    fields: &'b [TokenStream2],
    field_muts: &'b [TokenStream2],
) -> (
    impl Iterator<Item = TokenStream2> + 'b,
    impl Iterator<Item = TokenStream2> + 'b,
) {
    assert_eq!(props.len(), fields.len());

    // Perform field access override
    let fields = props.iter().zip(fields.iter()).map(|(p, f)| {
        if let Some(field_get) = &p.field.field {
            let ident = &p.field_quote;
            quote!(self.#ident.#field_get)
        } else {
            quote!(#f)
        }
    });

    let field_muts = props.iter().zip(field_muts.iter()).map(|(p, f)| {
        if let Some(field_get_mut) = &p.field.field_mut {
            let ident = &p.field_quote;
            quote!(self.#ident.#field_get_mut)
        } else {
            quote!(#f)
        }
    });

    (fields, field_muts)
}
