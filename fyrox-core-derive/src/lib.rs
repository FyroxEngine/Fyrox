mod inspect;
mod reflect;
mod visit;

use darling::FromDeriveInput;
use proc_macro::TokenStream;
use syn::{parse_macro_input, DeriveInput};

/// Implements `Visit` trait
///
/// User has to import `Visit`, `Visitor` and `VisitResult` to use this macro.
#[proc_macro_derive(Visit, attributes(visit))]
pub fn visit(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    TokenStream::from(visit::impl_visit(ast))
}

/// Implements `Inspect` trait
///
/// User has to import `Inspect` and `PropertyInfo` to use this macro.
#[proc_macro_derive(Inspect, attributes(inspect))]
pub fn inspect(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    TokenStream::from(inspect::impl_inspect(ast))
}

/// Implements `Reflect` trait
#[proc_macro_derive(Reflect, attributes(reflect))]
pub fn reflect(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    let mut ty_args = reflect::args::TypeArgs::from_derive_input(&ast).unwrap();
    ty_args.validate();

    let reflect_impl = reflect::impl_reflect(&ty_args);
    let prop_key_impl = reflect::impl_prop_constants(&ty_args);

    TokenStream::from(quote::quote! {
        #reflect_impl
        #prop_key_impl
    })
}

/// Implements `Reflect` by analyzing derive input, without adding property constants
///
/// This is used to implement the `Reflect` trait for external types.
#[proc_macro]
pub fn impl_reflect(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    let mut ty_args = reflect::args::TypeArgs::from_derive_input(&ast).unwrap();
    ty_args.validate();

    let reflect_impl = reflect::impl_reflect(&ty_args);

    TokenStream::from(reflect_impl)
}
