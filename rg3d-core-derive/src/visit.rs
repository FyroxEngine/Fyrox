use convert_case::{Case, Casing};
use proc_macro2::TokenStream as TokenStream2;
use quote::*;
use syn::*;

// implements `Visit` trait
pub fn impl_visit(ast: DeriveInput) -> TokenStream2 {
    match ast.data {
        Data::Struct(ref data) => self::impl_visit_struct(&ast, data),
        Data::Enum(ref _data) => todo!("add enum support for #[derive(Visit)]"),
        Data::Union(ref _union) => todo!("add union support for #[derive(Visit)]"),
    }
}

fn impl_visit_struct(ast: &DeriveInput, data: &DataStruct) -> TokenStream2 {
    let fields = match data.fields {
        Fields::Named(ref fields) => fields,
        Fields::Unnamed(ref _fields) => todo!("support unnamed fields"),
        Fields::Unit => todo!("support unit struct"),
    };

    // `self.field.visit(..);`
    let field_visits = fields.named.iter().map(|field| {
        let field_ident = field.ident.as_ref().unwrap_or_else(|| unreachable!());
        let field_name = format!("{}", field_ident).to_case(Case::UpperCamel);

        quote! {
            self.#field_ident.visit(#field_name, visitor)?;
        }
    });

    // `impl Visit`
    let ty_name = &ast.ident;
    let (impl_generics, ty_generics, where_clause) = ast.generics.split_for_impl();

    quote! {
        impl #impl_generics rg3d::core::visitor::Visit for #ty_name #ty_generics #where_clause {
            fn visit(&mut self, name: &str, visitor: &mut rg3d::core::visitor::Visitor) -> VisitResult {
                visitor.enter_region(name)?;

                #(#field_visits)*

                visitor.leave_region()
            }
        }
    }
}
