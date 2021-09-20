mod inspect;
mod visit;

use proc_macro::TokenStream;
use syn::{parse_macro_input, DeriveInput};

/// Implements `Visit` trait
///
/// User has to import `Visit`, `Visitor` and `VisitResult`.
#[proc_macro_derive(Visit, attributes(visit))]
pub fn visit(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    TokenStream::from(visit::impl_visit(ast))
}

/// Implements `Inspect` trait
///
/// User has to import `Inspect`.
#[proc_macro_derive(Inspect, attributes(inspect))]
pub fn inspect(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    TokenStream::from(inspect::impl_inspect(ast))
}
