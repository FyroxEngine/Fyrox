//! Derive input types defined with `darling`.
//!
//! They parse `#[attributes(..)]` syntax in a declartive style.

use darling::*;
use syn::*;

// pub type Data = ast::Data<VariantArgs, FieldArgs>;
pub type Fields = ast::Fields<FieldArgs>;

#[derive(FromDeriveInput)]
#[darling(attributes(inspect), supports(struct_any, enum_any))]
pub struct TypeArgs {
    pub ident: Ident,
    pub generics: Generics,
    pub data: ast::Data<VariantArgs, FieldArgs>,
}

/// Parsed from struct's or enum variant's field
///
/// NOTE: `#[derive(Inspect)]` is non-recursive by default.
#[derive(FromField, Clone, PartialEq)]
#[darling(attributes(inspect))]
pub struct FieldArgs {
    pub ident: Option<Ident>,

    pub ty: Type,

    /// `#[inspect(skip)]`
    ///
    /// Do not expose property info
    #[darling(default)]
    pub skip: bool,

    /// #[inspect(name = "<name>")]
    ///
    /// Name override for a field (default: Title Case)
    #[darling(default)]
    pub name: Option<String>,

    /// #[inspect(display_name = "<name>")]
    ///
    /// A human-readable name.
    #[darling(default)]
    pub display_name: Option<String>,

    /// `#[inspect(getter = "<path>")]`
    ///
    /// Convert the field reference to another reference
    #[darling(default)]
    pub getter: Option<String>,

    /// `#[inspect(read_only)]`
    ///
    /// The field is not meant to be edited.
    #[darling(default)]
    pub read_only: bool,

    /// `#[inspect(min_value = "0.0")]`
    ///
    /// Minimal value of the field. Works only for numeric fields!
    #[darling(default)]
    pub min_value: Option<f64>,

    /// `#[inspect(max_value = "1.0")]`
    ///
    /// Maximal value of the field. Works only for numeric fields!
    #[darling(default)]
    pub max_value: Option<f64>,

    /// `#[inspect(step = "0.1")]`
    ///
    /// Increment/decrement step of the field. Works only for numeric fields!
    #[darling(default)]
    pub step: Option<f64>,

    /// `#[inspect(precision = "3")]`
    ///
    /// Maximum amount of decimal places for a numeric property.
    #[darling(default)]
    pub precision: Option<usize>,

    /// `#[inspect(description = "This is a property description.")]`
    ///
    /// Description of the property.
    #[darling(default)]
    pub description: Option<String>,

    /// `#[inspect(getter = "<method_name>")]`
    ///
    /// True if the value has been modified.
    #[darling(default)]
    pub is_modified: Option<String>,
}

#[derive(FromVariant)]
#[darling(attributes(inspect))]
pub struct VariantArgs {
    pub ident: Ident,
    pub fields: ast::Fields<FieldArgs>,
}
