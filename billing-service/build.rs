fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = std::path::PathBuf::from(std::env::var("OUT_DIR")?);

    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .file_descriptor_set_path(out_dir.join("billing_descriptor.bin"))
        .compile_protos(&["../proto/micros/billing/v1/billing.proto"], &["../proto"])?;

    println!("cargo:rerun-if-changed=../proto/micros/billing/v1/billing.proto");

    Ok(())
}
