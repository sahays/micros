fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = std::path::PathBuf::from(std::env::var("OUT_DIR")?);

    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .file_descriptor_set_path(out_dir.join("invoicing_descriptor.bin"))
        .compile_protos(
            &["../proto/micros/invoicing/v1/invoicing.proto"],
            &["../proto"],
        )?;

    println!("cargo:rerun-if-changed=../proto/micros/invoicing/v1/invoicing.proto");

    Ok(())
}
