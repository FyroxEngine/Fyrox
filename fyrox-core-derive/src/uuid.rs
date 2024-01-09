use darling::*;
use proc_macro2::TokenStream as TokenStream2;
use quote::*;
use syn::*;

#[derive(FromDeriveInput)]
#[darling(attributes(type_uuid), supports(struct_any, enum_any))]
pub struct TypeArgs {
    pub ident: Ident,
    pub generics: Generics,
    pub id: String,
}

pub fn impl_type_uuid_provider(ast: DeriveInput) -> TokenStream2 {
    let ty_args = TypeArgs::from_derive_input(&ast).unwrap();
    let ty_ident = &ty_args.ident;
    let id = &ty_args.id;

    let (impl_generics, ty_generics, where_clause) = ty_args.generics.split_for_impl();

    quote! {
        impl #impl_generics TypeUuidProvider for #ty_ident #ty_generics #where_clause {
            fn type_uuid() -> Uuid {
                uuid!(#id)
            }
        }
    }
}
