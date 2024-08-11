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
use syn::*;

#[derive(FromDeriveInput)]
#[darling(attributes(visit), supports(struct_any, enum_any))]
pub struct TypeArgs {
    pub ident: Ident,
    pub generics: Generics,
    pub data: ast::Data<VariantArgs, FieldArgs>,
    #[darling(default)]
    pub optional: bool,
}

/// Parsed from struct's or enum variant's field
#[derive(FromField, Clone)]
#[darling(attributes(visit))]
pub struct FieldArgs {
    pub ident: Option<Ident>,
    // pub vis: Visibility,
    pub ty: Type,
    // pub attrs: Vec<Attribute>,
    // ---
    /// `#[visit(skip)]`
    ///
    /// Skip on read and write
    #[darling(default)]
    pub skip: bool,

    /// `#[visit(rename = "..")]`
    ///
    /// Force this name
    #[darling(default)]
    pub rename: Option<String>,

    /// `#[visit(optional)]`
    ///
    /// Ignore missing field
    #[darling(default)]
    pub optional: bool,
}

#[derive(FromVariant)]
#[darling(attributes(inspect))]
pub struct VariantArgs {
    pub ident: Ident,
    pub fields: ast::Fields<FieldArgs>,
}
