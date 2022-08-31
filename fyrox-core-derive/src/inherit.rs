//! Implements `DirectlyInheritableEntity` trait

// not using `darling` (right now)

use proc_macro2::TokenStream as TokenStream2;
use quote::*;
use syn::*;

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

    let fields = fields
        .named
        .iter()
        .filter_map(|f| {
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
        })
        .collect::<Vec<_>>();

    let field_idents = fields.iter().map(|f| &f.ident).collect::<Vec<_>>();

    let ty_ident = &ast.ident;

    let generics = self::impl_generics(ast, &fields);
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

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

fn impl_generics(ast: &DeriveInput, fields: &[&Field]) -> Generics {
    let mut generics = ast.generics.clone();

    // Add where clause for every inherited field
    generics.make_where_clause().predicates.extend(
        fields
            .iter()
            .map(|f| &f.ty)
            .map::<WherePredicate, _>(|ty| parse_quote! { #ty: InheritableVariable }),
    );

    generics
}
