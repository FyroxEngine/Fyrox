use darling::ast;
use fxhash::FxHashSet;
use proc_macro2::TokenStream as TokenStream2;
use quote::*;
use syn::*;

use convert_case::*;

use crate::visit::args;

pub fn create_impl(
    ty_args: &args::TypeArgs,
    field_args: impl Iterator<Item = args::FieldArgs>,
    impl_body: TokenStream2,
) -> TokenStream2 {
    let ty_ident = &ty_args.ident;
    let generics = self::create_impl_generics(&ty_args.generics, field_args);
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    quote! {
        impl #impl_generics Visit for #ty_ident #ty_generics #where_clause {
            fn visit(
                &mut self,
                name: &str,
                visitor: &mut Visitor,
            ) -> VisitResult {
                #impl_body
            }
        }
    }
}

fn create_impl_generics(
    generics: &Generics,
    field_args: impl Iterator<Item = args::FieldArgs>,
) -> Generics {
    let mut generics = generics.clone();

    // Add where clause for every visited field
    generics.make_where_clause().predicates.extend(
        field_args
            .filter(|f| !f.skip)
            .map(|f| f.ty)
            .map::<WherePredicate, _>(|ty| parse_quote! { #ty: Visit }),
    );

    generics
}

/// `<prefix>field.visit("name", visitor);`
pub fn create_field_visits<'a>(
    // None or `f` when bindings tuple variants. NOTE: We can't use `prefix: Ident`
    prefix: Option<Ident>,
    fields: impl Iterator<Item = &'a args::FieldArgs>,
    field_style: ast::Style,
) -> Vec<TokenStream2> {
    if field_style == ast::Style::Unit {
        // `Unit` (struct/enum variant) has no field to visit.
        // We won't even enter this region:
        return vec![];
    }

    let visit_args = fields
        .filter(|field| !field.skip)
        .enumerate()
        .map(|(field_index, field)| {
            let (ident, name) = match field_style {
                // `NamedFields { a: f32, .. }`
                ast::Style::Struct => {
                    let ident = field.ident.as_ref().unwrap_or_else(|| unreachable!());

                    (
                        quote!(#ident),
                        format!("{}", ident).to_case(Case::UpperCamel),
                    )
                }
                // `Tuple(f32, ..)`
                ast::Style::Tuple => {
                    let ident = Index::from(field_index);

                    let ident = match prefix {
                        Some(ref prefix) => {
                            let ident = format_ident!("{}{}", prefix, ident);
                            quote!(#ident)
                        }
                        None => quote!(#ident),
                    };

                    (ident, format!("{}", field_index))
                }
                ast::Style::Unit => unreachable!(),
            };

            let name = match &field.rename {
                Some(new_name) => {
                    assert!(
                        !new_name.is_empty(),
                        "renaming to empty string doesn't make sense!"
                    );
                    // overwrite the field name with the specified name:
                    new_name.clone()
                }
                None => name,
            };

            (ident, name, field.optional)
        })
        .collect::<Vec<_>>();

    let mut no_dup = FxHashSet::default();
    for name in visit_args.iter().map(|(_, name, _)| name) {
        if !no_dup.insert(name) {
            panic!("duplicate visiting names detected!");
        }
    }

    visit_args
        .iter()
        .map(|(ident, name, optional)| {
            if *optional {
                quote! {
                    #ident.visit(#name, visitor).ok();
                }
            } else {
                quote! {
                    #ident.visit(#name, visitor)?;
                }
            }
        })
        .collect::<Vec<_>>()
}
