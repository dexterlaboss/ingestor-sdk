use std::path::PathBuf;
use serde_json::Value;
use std::process::Command;

fn get_third_party_crate_path(crate_name: &str) -> Option<PathBuf> {
    let output = Command::new("cargo")
        .arg("metadata")
        .arg("--format-version")
        .arg("1")
        .output()
        .ok()?;

    let metadata: Value = serde_json::from_slice(&output.stdout).ok()?;

    metadata["packages"]
        .as_array()?
        .iter()
        .find(|package| package["name"] == crate_name)
        .and_then(|package| package["manifest_path"].as_str().map(PathBuf::from))
        .and_then(|manifest_path| manifest_path.parent().map(|p| p.to_path_buf())) // ✅ FIXED HERE
}

fn main() -> Result<(), std::io::Error> {
    let solana_proto_path = get_third_party_crate_path("solana-storage-proto")
        .expect("Failed to locate solana-storage-proto crate");

    const PROTOC_ENVAR: &str = "PROTOC";
    if std::env::var(PROTOC_ENVAR).is_err() {
        #[cfg(not(windows))]
        std::env::set_var(PROTOC_ENVAR, protobuf_src::protoc());
    }

    let proto_base_path = PathBuf::from("proto");
    let proto_file = proto_base_path.join("confirmed_tx.proto");
    let solana_proto_base_path = solana_proto_path.join("proto");
    println!("cargo:rerun-if-changed={}", proto_file.display());

    tonic_build::configure()
        .build_client(false)
        .build_server(false)
        .extern_path(
            ".solana.storage.ConfirmedBlock.ConfirmedTransaction",
            "::solana_storage_proto::convert::generated::ConfirmedTransaction",
        )
        .extern_path(
            ".solana.storage.ConfirmedBlock.UnixTimestamp",
            "::solana_storage_proto::convert::generated::UnixTimestamp",
        )
        .compile(&[proto_file], &[proto_base_path, solana_proto_base_path])?;

    Ok(())
}