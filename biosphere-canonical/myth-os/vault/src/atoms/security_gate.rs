// VAULT-ATOM-09: Security Gatekeeper — token-based access validation
use std::collections::HashSet;
use std::sync::{Arc, RwLock};

#[derive(Default)]
pub struct SecurityGate {
    valid_tokens: Arc<RwLock<HashSet<String>>>,
}

impl SecurityGate {
    pub fn issue_token(&self, token: impl Into<String>) {
        self.valid_tokens.write().unwrap().insert(token.into());
    }

    pub fn revoke_token(&self, token: &str) {
        self.valid_tokens.write().unwrap().remove(token);
    }

    pub fn authorize(&self, token: &str) -> bool {
        self.valid_tokens.read().unwrap().contains(token)
    }
}
