use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::DeriveInput;

pub(crate) fn impl_script_message_payload(ast: DeriveInput) -> TokenStream2 {
    let ident = &ast.ident;
    quote! {
        impl ScriptMessagePayload for #ident {
            fn as_any_ref(&self) -> &dyn std::any::Any {
                self
            }

            fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
                self
            }
        }
    }
}
