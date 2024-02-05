mod component;
mod reflect;
mod script;
mod uuid;
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

/// Implements `TypeUuidProvider` trait
///
/// User has to import `TypeUuidProvider` trait to use this macro.
#[proc_macro_derive(TypeUuidProvider, attributes(type_uuid))]
pub fn type_uuid(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    TokenStream::from(uuid::impl_type_uuid_provider(ast))
}

/// Implements `ScriptSourcePathProvider` trait
///
/// User has to import `ScriptSourcePathProvider` trait to use this macro.
#[proc_macro_derive(ScriptSourcePathProvider)]
pub fn script_source_path(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    TokenStream::from(script::impl_script_source_path_provider(ast))
}

/// Implements `ComponentProvider` trait
///
/// User has to import `ComponentProvider` trait to use this macro.
#[proc_macro_derive(ComponentProvider, attributes(component))]
pub fn component(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    TokenStream::from(component::impl_type_uuid_provider(ast))
}
