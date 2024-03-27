use darling::*;
use proc_macro2::{TokenStream as TokenStream2, TokenStream};
use quote::*;
use syn::*;

#[derive(FromDeriveInput)]
#[darling(attributes(component), supports(struct_any, enum_any))]
pub struct TypeArgs {
    pub ident: Ident,
    pub generics: Generics,
    pub data: ast::Data<VariantArgs, FieldArgs>,
}

#[derive(FromField, Clone)]
#[darling(attributes(component))]
pub struct FieldArgs {
    pub ident: Option<Ident>,
    pub ty: Type,
    #[darling(default)]
    pub include: bool,
    #[darling(default)]
    pub path: Option<String>,
    #[darling(default)]
    pub dest_type: Option<Type>,
}

#[derive(FromVariant)]
#[darling(attributes(component))]
#[allow(dead_code)] // TODO: Add support for enum variants.
pub struct VariantArgs {
    pub ident: Ident,
    pub fields: ast::Fields<FieldArgs>,
}

pub fn impl_type_uuid_provider(ast: DeriveInput) -> TokenStream2 {
    let ty_args = TypeArgs::from_derive_input(&ast).unwrap();
    match &ty_args.data {
        ast::Data::Struct(ref field_args) => impl_type_uuid_provider_struct(&ty_args, field_args),
        ast::Data::Enum(_) => unimplemented!(),
    }
}

fn create_field_components(
    // false if enum variant
    is_struct: bool,
    fields: &[FieldArgs],
    field_style: ast::Style,
    mutable: bool,
) -> Vec<TokenStream2> {
    if field_style == ast::Style::Unit {
        // `Unit` struct/enum variant has no components.
        return Default::default();
    }

    let prefix = if is_struct {
        if mutable {
            Some(quote!(&mut self.))
        } else {
            Some(quote!(&self.))
        }
    } else {
        None
    };

    fields
        .iter()
        .filter(|arg| arg.include)
        .map(|arg| {
            let ident = &arg.ident;
            let ty = &arg.ty;
            if let Some(path) = &arg.path {
                let path: TokenStream = parse_str(path).unwrap();

                if let Some(dest_type) = arg.dest_type.as_ref() {
                    quote! {
                        if type_id == std::any::TypeId::of::<#dest_type>() {
                            return Some(#prefix #path);
                        }
                    }
                } else {
                    quote! {
                        if type_id == std::any::TypeId::of::<#ty>() {
                            return Some(#prefix #path);
                        }
                    }
                }
            } else {
                quote! {
                    if type_id == std::any::TypeId::of::<#ty>() {
                        return Some(#prefix #ident);
                    }
                }
            }
        })
        .collect::<Vec<_>>()
}

fn impl_type_uuid_provider_struct(
    ty_args: &TypeArgs,
    field_args: &ast::Fields<FieldArgs>,
) -> TokenStream2 {
    let (field_refs, field_ref_muts) = if field_args.style == ast::Style::Unit {
        (quote! {}, quote! {})
    } else {
        let field_refs = create_field_components(true, &field_args.fields, field_args.style, false);
        let field_ref_muts =
            create_field_components(true, &field_args.fields, field_args.style, true);
        (quote! { #(#field_refs)* }, quote! { #(#field_ref_muts)* })
    };

    let ty_ident = &ty_args.ident;
    let (impl_generics, ty_generics, where_clause) = ty_args.generics.split_for_impl();

    quote! {
        impl #impl_generics ComponentProvider for #ty_ident #ty_generics #where_clause {
            fn query_component_ref(&self, type_id: std::any::TypeId) -> Option<&dyn std::any::Any> {
                if type_id == std::any::TypeId::of::<Self>() {
                    return Some(self);
                }

                #field_refs

                None
            }

            fn query_component_mut(
                &mut self,
                type_id: std::any::TypeId,
            ) -> Option<&mut dyn std::any::Any> {
                if type_id == std::any::TypeId::of::<Self>() {
                    return Some(self);
                }

                #field_ref_muts

                None
            }
        }
    }
}
