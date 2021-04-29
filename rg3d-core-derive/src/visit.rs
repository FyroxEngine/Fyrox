mod args;

use convert_case::{Case, Casing};
use darling::*;
use proc_macro2::TokenStream as TokenStream2;
use quote::*;
use syn::*;

// implements `Visit` trait
pub fn impl_visit(ast: DeriveInput) -> TokenStream2 {
    match ast.data {
        Data::Struct(ref _data) => {
            let struct_args = args::StructArgs::from_derive_input(&ast).unwrap();
            self::impl_visit_struct(struct_args)
        }
        Data::Enum(ref _data) => todo!("add enum support for #[derive(Visit)]"),
        Data::Union(ref _union) => todo!("add union support for #[derive(Visit)]"),
    }
}

fn impl_visit_struct(args: args::StructArgs) -> TokenStream2 {
    let field_args = match args.data {
        ast::Data::Struct(ref field_args) => field_args,
        ast::Data::Enum(ref _variants) => unreachable!(),
    };

    // we only accept struct with named fields (for now)
    assert_eq!(
        field_args.style,
        ast::Style::Struct,
        "add tuple/unit field support for #[derive(Visit)]"
    );

    // `self.field.visit(..);`
    let field_visits = field_args.fields.iter().filter_map(|field| {
        if field.skip {
            return None;
        }

        let field_ident = field.ident.as_ref().unwrap_or_else(|| unreachable!());
        let field_name = format!("{}", field_ident).to_case(Case::UpperCamel);

        Some(quote! {
            self.#field_ident.visit(#field_name, visitor)?;
        })
    });

    // `impl Visit`
    let ty_name = &args.ident;
    let (impl_generics, ty_generics, where_clause) = args.generics.split_for_impl();

    quote! {
        impl #impl_generics rg3d::core::visitor::Visit for #ty_name #ty_generics #where_clause {
            fn visit(
                &mut self,
                name: &str,
                visitor: &mut rg3d::core::visitor::Visitor,
            ) -> rg3d::core::visitor::VisitResult {
                visitor.enter_region(name)?;

                #(#field_visits)*

                visitor.leave_region()
            }
        }
    }
}
