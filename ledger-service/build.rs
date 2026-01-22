fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = std::path::PathBuf::from(std::env::var("OUT_DIR")?);

    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .file_descriptor_set_path(out_dir.join("ledger_descriptor.bin"))
        .compile_protos(&["../proto/micros/ledger/v1/ledger.proto"], &["../proto"])?;

    println!("cargo:rerun-if-changed=../proto/micros/ledger/v1/ledger.proto");

    Ok(())
}
