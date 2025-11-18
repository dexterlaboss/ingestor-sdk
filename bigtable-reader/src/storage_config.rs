
use {
    solana_bigtable_shared::{
        CredentialType,
        DEFAULT_INSTANCE_NAME,
        DEFAULT_APP_PROFILE_ID,
    },
};

#[derive(Debug)]
pub struct LedgerStorageConfig {
    pub read_only: bool,
    pub timeout: Option<std::time::Duration>,
    pub credential_type: CredentialType,
    pub instance_name: String,
    pub app_profile_id: String,
}

impl Default for LedgerStorageConfig {
    fn default() -> Self {
        Self {
            read_only: true,
            timeout: None,
            credential_type: CredentialType::Filepath(None),
            instance_name: DEFAULT_INSTANCE_NAME.to_string(),
            app_profile_id: DEFAULT_APP_PROFILE_ID.to_string(),
        }
    }
}