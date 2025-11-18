#![allow(clippy::integer_arithmetic)]

pub mod access_token;

pub mod root_ca_certificate;

pub const DEFAULT_INSTANCE_NAME: &str = "solana-ledger";
pub const DEFAULT_APP_PROFILE_ID: &str = "default";

#[derive(Debug)]
pub enum CredentialType {
    Filepath(Option<String>),
    Stringified(String),
}


#[allow(clippy::derive_partial_eq_without_eq)]
pub mod google {
    mod rpc {
        include!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        concat!("/proto/google.rpc.rs")
        ));
    }
    pub mod bigtable {
        pub mod v2 {
            include!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            concat!("/proto/google.bigtable.v2.rs")
            ));
        }
    }
}