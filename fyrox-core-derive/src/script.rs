use darling::*;
use proc_macro2::TokenStream as TokenStream2;
use quote::*;
use syn::*;

#[derive(FromDeriveInput)]
#[darling(supports(struct_any, enum_any))]
pub struct TypeArgs {
    pub ident: Ident,
}

pub fn impl_script_source_path_provider(ast: DeriveInput) -> TokenStream2 {
    let ty_args = TypeArgs::from_derive_input(&ast).unwrap();
    let ty_ident = &ty_args.ident;

    quote! {
        impl ScriptSourcePathProvider for #ty_ident {
            fn script_source_path() -> &'static str {
                file!()
            }
        }
    }
}
