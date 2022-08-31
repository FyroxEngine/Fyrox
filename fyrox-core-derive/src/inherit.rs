//! Implements `DirectlyInheritableEntity` trait

// not using `darling` (right now)

use proc_macro2::TokenStream as TokenStream2;
use syn::*;
use quote::quote;

pub fn impl_inherit(ast: DeriveInput) -> TokenStream2 {
    match &ast.data {
        Data::Struct(ref s) => self::impl_struct(&ast, s),
        _ => todo!(),
    }
}

fn impl_struct(ast: &DeriveInput, s: &DataStruct) -> TokenStream2 {
    let fields = match &s.fields {
        Fields::Named(xs) => xs,
        _ => todo!(),
    };

    let fields = fields.named.iter().filter_map(|f| {
        // find `Meta:Path` of `#[inherit]`
        f.attrs.iter().find_map(|a| {
            let meta = match a.parse_meta() {
                Ok(meta) => meta,
                Err(_) => return None,
            };

            let path = match meta {
                Meta::Path(p) => p,
                _ => return None,
            };

            let segment = path.segments.first().unwrap();
            if segment.ident == "inherit" {
                Some(f)
            } else {
                None
            }
        })
    });

    let field_idents = fields.map(|f| &f.ident).collect::<Vec<_>>();

    let ty_ident = &ast.ident;
    let (impl_generics, ty_generics, where_clause) = ast.generics.split_for_impl();

    quote! {
        impl #impl_generics DirectlyInheritableEntity for #ty_ident #ty_generics #where_clause {
            fn inheritable_properties_ref(&self) -> Vec<&dyn InheritableVariable> {
                vec![
                    #( &self.#field_idents, )*
                ]
            }

            fn inheritable_properties_mut(&mut self) -> Vec<&mut dyn InheritableVariable> {
                vec![
                    #( &mut self.#field_idents, )*
                ]
            }
        }
    }
}
