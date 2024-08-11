// Copyright (c) 2019-present Dmitry Stepanov and Fyrox Engine contributors.
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

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
