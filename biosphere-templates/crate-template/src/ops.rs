use crate::{domain::ExampleType, error::ModuleError};

// ── Mutation operations ─────────────────────────────────────────────────────
//
// Convention:
//   - Every mutation returns Result<&T, ModuleError> or Result<(), ModuleError>
//   - Check Capacity Law before adding children
//   - Check lifecycle (is_mutable()) before any structural change
//   - Validate heraldry on the incoming item
//   - Set B-DNA on every new entity

/// Example: validate an ExampleType according to module rules.
pub fn validate(item: &ExampleType) -> Vec<ModuleError> {
    let mut errors = Vec::new();

    if item.name.is_empty() {
        errors.push(ModuleError::Validation {
            message: format!("Entity '{}' has an empty name", item.id),
        });
    }

    // TODO: add capacity, heraldry, lifecycle, B-DNA checks

    errors
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_name_fails_validation() {
        let item = ExampleType { id: "test-id".into(), name: String::new() };
        let errors = validate(&item);
        assert!(!errors.is_empty());
    }

    #[test]
    fn valid_item_passes() {
        let item = ExampleType::new("Valid Item".into());
        let errors = validate(&item);
        assert!(errors.is_empty());
    }
}
