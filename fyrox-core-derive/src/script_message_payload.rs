use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::DeriveInput;

pub(crate) fn impl_script_message_payload(ast: DeriveInput) -> TokenStream2 {
    let ident = &ast.ident;
    quote! {
        impl ScriptMessagePayload for #ident {

        }
    }
}
