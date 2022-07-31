//! Derive input types defined with `darling`.

use darling::*;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::*;

pub type Fields = ast::Fields<FieldArgs>;

#[derive(FromDeriveInput)]
#[darling(attributes(reflect), supports(struct_any, enum_any))]
pub struct TypeArgs {
    pub ident: Ident,
    pub generics: Generics,
    pub data: ast::Data<VariantArgs, FieldArgs>,

    /// Hides all fields and creates an empty impl
    #[darling(default)]
    pub hide_all: bool,

    /// Custom `Reflect` impl type boundary. It's useful if you mark some field as `deref` or
    /// `hidden` but the type needs to be `Refelct` to implement `Reflect`.
    #[darling(default)]
    pub bounds: Option<Vec<WherePredicate>>,

    #[darling(default, rename = "ReflectList")]
    pub impl_as_list: bool,
}

impl TypeArgs {
    pub fn validate(&mut self) {
        match &mut self.data {
            ast::Data::Enum(vs) => {
                vs.iter_mut()
                    .for_each(|v| v.fields.fields.iter_mut().for_each(|f| f.validate()));
            }
            ast::Data::Struct(s) => {
                s.fields.iter_mut().for_each(|f| f.validate());
            }
        }
    }

    /// Creates impl [`Generics`] adding bounds
    pub fn impl_generics(&self) -> Generics {
        let mut generics = self.generics.clone();

        let clause = generics.make_where_clause();

        clause.predicates.push(parse_quote! { Self: 'static });
        if let Some(bounds) = &self.bounds {
            clause.predicates.extend(bounds.iter().cloned());
        }

        if self.hide_all {
            return generics;
        }

        // Add where clause for every reflectable field
        let fields: Box<dyn Iterator<Item = &FieldArgs>> = match &self.data {
            ast::Data::Struct(data) => Box::new(data.fields.iter()),
            ast::Data::Enum(variants) => Box::new(variants.iter().flat_map(|v| v.fields.iter())),
        };

        clause.predicates.extend(
            fields
                .filter(|f| !(f.hidden || f.deref || f.field.is_some()))
                .map(|f| &f.ty)
                .map::<WherePredicate, _>(|ty| parse_quote! { #ty: Reflect }),
        );

        generics
    }

    pub fn as_list_impl(&self) -> TokenStream2 {
        if !self.impl_as_list {
            return quote!();
        }

        quote! {
            fn as_list(&self) -> Option<&dyn ReflectList> {
                Some(self)
            }

            fn as_list_mut(&mut self) -> Option<&mut dyn ReflectList> {
                Some(self)
            }
        }
    }
}

#[derive(FromField, Clone, PartialEq, Eq)]
#[darling(attributes(reflect))]
pub struct FieldArgs {
    pub ident: Option<Ident>,
    pub ty: Type,

    /// `#[reflect(name = .. )]`
    ///
    /// Property name override for a field (default: snake_case)
    #[darling(default)]
    pub name: Option<String>,

    /// `#[reflect(hidden)]`
    ///
    /// Do not expose the property key
    #[darling(default)]
    pub hidden: bool,

    /// `#[reflect(deref)]`
    ///
    /// Sets `field` and `field_mut` attributes with `deref()` and `deref_mut()`
    #[darling(default)]
    pub deref: bool,

    /// `#[reflect(field = "<method call>")]
    ///
    /// Implement `Reflect::field` with the method call
    #[darling(default)]
    pub field: Option<Expr>,

    /// `#[reflect(field_mut = "<method call>")]
    ///
    /// Implement `Reflect::field_mut` with the method call
    #[darling(default)]
    pub field_mut: Option<Expr>,

    /// `#[reflect(setter = "<method name>")]
    ///
    /// **STRUCT-ONLY (for now)**
    ///
    /// Setter method name used in `Reflect::set_field`.
    /// Expected signature: `fn(&mut self, value: T)`
    #[darling(default)]
    pub setter: Option<Path>,
}

impl FieldArgs {
    pub fn validate(&mut self) {
        if self.deref {
            assert!(
                self.field.is_none() || self.field_mut.is_none(),
                "can't use both `deref` and `field` + `field_mut`"
            );
        }

        assert!(
            !(self.field.is_none() ^ self.field_mut.is_none()),
            "use both `field` and `field_mut`"
        );

        if self.deref {
            self.field = Some(parse_quote!(deref()));
            self.field_mut = Some(parse_quote!(deref_mut()));
        }
    }
}

#[derive(FromVariant)]
#[darling(attributes(reflect))]
pub struct VariantArgs {
    pub ident: Ident,
    pub fields: ast::Fields<FieldArgs>,
}
