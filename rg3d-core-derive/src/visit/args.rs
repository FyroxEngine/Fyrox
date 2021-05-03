use darling::*;
use syn::*;

#[derive(FromDeriveInput)]
#[darling(attributes(visit), supports(struct_any))]
pub struct StructArgs {
    pub ident: Ident,
    // pub vis: Visibility,
    pub generics: Generics,
    pub data: ast::Data<util::Ignored, FieldArgs>,
    // attrs: Vec<Attribute>
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
    /// `#[visit(skip)]`: skip on read and write
    #[darling(default)]
    pub skip: bool,
}

/// Collect [`FieldArgs`] manually from [`syn::Variant`]
pub fn field_args_from_variant(variant: &Variant) -> (Vec<FieldArgs>, ast::Style) {
    let (fields, style): (Vec<_>, _) = match &variant.fields {
        Fields::Named(fields) => (fields.named.iter().collect(), ast::Style::Struct),
        Fields::Unnamed(fields) => (fields.unnamed.iter().collect(), ast::Style::Tuple),
        Fields::Unit => (vec![], ast::Style::Unit),
    };

    let fields = fields
        .iter()
        .map(|f| self::FieldArgs::from_field(f).unwrap())
        .collect::<Vec<_>>();

    (fields, style)
}
