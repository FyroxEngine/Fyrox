// VAULT-ATOM-12: Portal Translator — generic cross-vault serialization.
//
// Packs any Serialize value to JSON bytes for cross-vault migration,
// and unpacks JSON bytes back to any DeserializeOwned type.
// The Vault is agnostic to the type — it just moves bytes.

use crate::error::VaultResult;
use serde::{de::DeserializeOwned, Serialize};

pub struct PortalTranslator;

impl PortalTranslator {
    pub fn pack<T: Serialize>(value: &T) -> VaultResult<Vec<u8>> {
        Ok(serde_json::to_vec(value)?)
    }

    pub fn unpack<T: DeserializeOwned>(data: &[u8]) -> VaultResult<T> {
        Ok(serde_json::from_slice(data)?)
    }
}
