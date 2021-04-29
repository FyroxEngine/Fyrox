mod args;

use convert_case::{Case, Casing};
use darling::*;
use proc_macro2::TokenStream as TokenStream2;
use quote::*;
use syn::*;

// impl `#[derive(Visit)]` for `struct` or `enum`
pub fn impl_visit(ast: DeriveInput) -> TokenStream2 {
    match ast.data {
        Data::Struct(ref _data) => {
            let struct_args = args::StructArgs::from_derive_input(&ast).unwrap();
            self::impl_visit_struct(struct_args)
        }
        Data::Enum(ref data) => self::impl_visit_enum(&ast, data),
        Data::Union(ref _union) => todo!("add union support for #[derive(Visit)]"),
    }
}

fn create_generics<'a>(
    generics: &Generics,
    field_args: impl Iterator<Item = &'a args::FieldArgs>,
) -> Generics {
    let mut generics = generics.clone();

    // Add where clause for every visited field
    generics.make_where_clause().predicates.extend(
        field_args
            .filter(|f| !f.skip)
            .map(|f| &f.ty)
            .map::<WherePredicate, _>(|ty| parse_quote! { #ty: Visit }),
    );

    generics
}

// `impl Visit` for struct
fn impl_visit_impl<'a>(
    ty_ident: &Ident,
    generics: &Generics,
    field_args: impl Iterator<Item = &'a args::FieldArgs>,
    visit_body: TokenStream2,
) -> TokenStream2 {
    let generics = self::create_generics(generics, field_args);
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    quote! {
        impl #impl_generics Visit for #ty_ident #ty_generics #where_clause {
            fn visit(
                &mut self,
                #[allow(unused)]
                name: &str,
                #[allow(unused)]
                visitor: &mut Visitor,
            ) -> VisitResult {
                #visit_body
            }
        }
    }
}

/// `<prefix>field.visit("name", visitor);`
fn create_field_visits<'a>(
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

    fields
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

            quote! {
                #ident.visit(#name, visitor)?;
            }
        })
        .collect::<Vec<_>>()
}

/// impl `Visit` for `struct`
fn impl_visit_struct(args: args::StructArgs) -> TokenStream2 {
    let field_args = args.data.take_struct().unwrap_or_else(|| unreachable!());

    self::impl_visit_impl(
        // impl block parameters:
        &args.ident,
        &args.generics,
        field_args.iter(),
        // visit function body:
        if field_args.style.clone() == ast::Style::Unit {
            quote! { Ok(()) }
        } else {
            // `field.visit(..);` parts
            let field_visits =
                self::create_field_visits(None, field_args.fields.iter(), field_args.style);

            quote! {
                visitor.enter_region(name)?;
                #(self.#field_visits)*
                visitor.leave_region()
            }
        },
    )
}

/// impl `Visit` for `enum`
fn impl_visit_enum(ast: &DeriveInput, data: &DataEnum) -> TokenStream2 {
    let ty_ident = &ast.ident;
    let ty_name = format!("{}", ty_ident);

    // variant ID = variant index
    let id_type = quote!(u32);

    // `fn id(&self) -> u32`
    let fn_id = {
        let matchers = data
            .variants
            .iter()
            .enumerate()
            .map(|(variant_index, variant)| {
                let variant_index = variant_index as u32;
                let variant_ident = &variant.ident;

                match variant.fields {
                    Fields::Named(_) => quote! {
                        #ty_ident::#variant_ident { .. } => #variant_index,
                    },
                    Fields::Unnamed(_) => {
                        let idents = (0..variant.fields.len()).map(|__| quote!(_));

                        quote! {
                            #ty_ident::#variant_ident(#(#idents),*) => #variant_index,
                        }
                    }
                    Fields::Unit => quote! {
                        #ty_ident::#variant_ident => #variant_index,
                    },
                }
            });

        quote! {
            fn id(me: &#ty_ident) -> #id_type {
                match me {
                    #(#matchers)*
                    _ => unreachable!("Unable to get ID from enum variant") ,
                }
            }
        }
    };

    // `fn from_id(id: u32) -> Result<Self, String>`
    let fn_from_id = {
        // `<variant_index> => Ok(TypeName::Variant(Default::default())),
        let matchers = data
            .variants
            .iter()
            .enumerate()
            .map(|(variant_index, variant)| {
                let variant_index = variant_index as u32;
                let variant_ident = &variant.ident;

                // create default value of this variant
                let default = match &variant.fields {
                    Fields::Named(fields) => {
                        let defaults = fields.named.iter().map(|field| {
                            let field_ident = &field.ident;
                            quote! {
                                #field_ident: Default::default(),
                            }
                        });

                        quote! {
                            #ty_ident::#variant_ident {
                                #(#defaults)*
                            },
                        }
                    }
                    Fields::Unnamed(fields) => {
                        let defaults = fields
                            .unnamed
                            .iter()
                            .map(|_| quote! { Default::default(), });

                        quote! {
                            #ty_ident::#variant_ident(#(#defaults)*),
                        }
                    }
                    Fields::Unit => quote! {
                        #ty_ident::#variant_ident
                    },
                };

                quote! {
                    id if id == #variant_index => Ok(#default),
                }
            });

        quote! {
            fn from_id(id: #id_type) -> std::result::Result<#ty_ident, String> {
                match id {
                    #(#matchers)*
                    _ => Err(format!("Unknown ID for type `{}`: `{}`", #ty_name, id)),
                }
            }
        }
    };

    // visit every field of each variant
    let variant_visits = data.variants.iter().map(|variant| {
        let (fields, style): (Vec<_>, _) = match &variant.fields {
            Fields::Named(fields) => (fields.named.iter().collect(), ast::Style::Struct),
            Fields::Unnamed(fields) => (fields.unnamed.iter().collect(), ast::Style::Tuple),
            Fields::Unit => (vec![], ast::Style::Unit),
        };

        let fields = fields
            .iter()
            .map(|f| args::FieldArgs::from_field(f).unwrap())
            .collect::<Vec<_>>();

        let variant_ident = &variant.ident;

        match style {
            ast::Style::Struct => {
                let field_visits = self::create_field_visits(None, fields.iter(), style);

                let idents = fields.iter().map(|field| {
                    let ident = &field.ident;
                    quote!(#ident)
                });

                quote! {
                    #ty_ident::#variant_ident { #(#idents),* } => {
                        #(#field_visits)*
                    },
                }
            }
            ast::Style::Tuple => {
                let field_visits = self::create_field_visits(parse_quote!(f), fields.iter(), style);

                let idents = (0..fields.len()).map(|i| format_ident!("f{}", Index::from(i)));

                quote! {
                    #ty_ident::#variant_ident(#(#idents),*) => {
                        #(#field_visits)*
                    },
                }
            }
            ast::Style::Unit => quote! {
                #ty_ident::#variant_ident => {},
            },
        }
    });

    // self::impl_visit_impl(ty_ident, &ast.generics,

    // TODO: add generic bounds
    let (impl_generics, ty_generics, where_clause) = ast.generics.split_for_impl();

    // plain-enum-only support
    quote! {
        impl #impl_generics Visit for #ty_ident #ty_generics #where_clause {
            fn visit(
                &mut self,
                name: &str,
                visitor: &mut Visitor,
            ) -> VisitResult {
                visitor.enter_region(name)?;

                let mut id = id(self);
                id.visit("Id", visitor)?;

                if visitor.is_reading() {
                    *self = from_id(id)?;
                }

                match self {
                    #(#variant_visits)*
                }

                return visitor.leave_region();

                #fn_id

                #fn_from_id
            }
        }
    }
}
