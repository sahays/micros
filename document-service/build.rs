fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = std::path::PathBuf::from(std::env::var("OUT_DIR")?);

    // Compile document service protos with file descriptor set for reflection
    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .file_descriptor_set_path(out_dir.join("document_descriptor.bin"))
        .compile_protos(
            &["../proto/micros/document/v1/document.proto"],
            &["../proto"],
        )?;

    println!("cargo:rerun-if-changed=../proto/micros/document/v1/document.proto");

    Ok(())
}
