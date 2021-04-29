mod visit;

use proc_macro::TokenStream;
use syn::{parse_macro_input, DeriveInput};

/// Implements `rg3d::core::visitor::Visit`
#[proc_macro_derive(Visit, attributes(visit))]
pub fn visit(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    TokenStream::from(visit::impl_visit(ast))
}
