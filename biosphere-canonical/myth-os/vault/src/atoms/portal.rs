// VAULT-ATOM-12: Portal Translator — serialize/deserialize capsules for cross-vault migration
use crate::error::VaultResult;
use mythos::capsule::Capsule;

pub struct PortalTranslator;

impl PortalTranslator {
    pub fn pack(capsule: &Capsule) -> VaultResult<Vec<u8>> {
        Ok(serde_json::to_vec(capsule)?)
    }

    pub fn unpack(data: &[u8]) -> VaultResult<Capsule> {
        Ok(serde_json::from_slice(data)?)
    }
}
