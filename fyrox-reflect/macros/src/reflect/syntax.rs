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

//! Syntax helpers

use darling::*;
use proc_macro2::TokenStream as TokenStream2;
use quote::*;
use syn::*;

use crate::reflect::args;

#[derive(Clone)]
pub struct VariantSyntax<'a> {
    ty_ident: Ident,
    args: &'a args::VariantArgs,
}

impl<'a> VariantSyntax<'a> {
    pub fn new(ty_ident: Ident, args: &'a args::VariantArgs) -> Self {
        Self { ty_ident, args }
    }

    // Returns LHS of the syntax of pattern match
    // ```
    // match x {
    //     X::Struct { a, b, c } => { .. }
    // //  ~~~~~~~~~~~~~~~~~~~~~
    // }
    // ```
    pub fn matcher(&self) -> TokenStream2 {
        let VariantSyntax {
            ty_ident,
            args: variant,
        } = self;

        let variant_ident = &variant.ident;

        let field_idents = variant
            .fields
            .iter()
            .enumerate()
            .filter(|(_, f)| !f.hidden)
            .map(|(i, f)| self.field_match_ident(i, f));

        let fields = match variant.fields.style {
            ast::Style::Struct => {
                quote! {
                    { #( #field_idents ),* }
                }
            }
            ast::Style::Tuple => {
                quote! {
                    ( #( #field_idents ),* )
                }
            }
            ast::Style::Unit => quote!(),
        };

        quote! {
            #ty_ident::#variant_ident #fields
        }
    }

    // Returns syntax for binding an enum variant's field on match:
    // ```
    // match x {
    //     X::Struct { a, b, c } => { .. }
    //     //         ~~~ use "field_name"
    //
    //     X::Tuple(f0, f1, f2) => { .. }
    //     //       ~~~ use "f<index>"
    // }
    // ```
    pub fn field_match_ident(&self, i: usize, field: &args::FieldArgs) -> Ident {
        match self.args.fields.style {
            ast::Style::Struct => field.ident.clone().unwrap(),
            ast::Style::Tuple => {
                let i = Index::from(i);
                format_ident!("f{}", i)
            }
            ast::Style::Unit => {
                unreachable!()
            }
        }
    }
}
