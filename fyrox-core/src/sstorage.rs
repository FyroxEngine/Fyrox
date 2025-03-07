use crate::{
    uuid_provider,
    visitor::{Visit, VisitResult, Visitor},
};
pub use fyrox_utils::sstorage::*;

uuid_provider!(ImmutableString = "452caac1-19f7-43d6-9e33-92c2c9163332");

impl Visit for ImmutableString {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        // Serialize/deserialize as ordinary string.
        let mut string = self.get_string();
        string.visit(name, visitor)?;

        // Deduplicate on deserialization.
        if visitor.is_reading() {
            *self = get_singleton_sstorage().lock().insert(string);
        }

        Ok(())
    }
}
