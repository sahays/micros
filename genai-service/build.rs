fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = std::path::PathBuf::from(std::env::var("OUT_DIR")?);

    // Compile genai proto (server + client)
    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .file_descriptor_set_path(out_dir.join("genai_descriptor.bin"))
        .compile_protos(&["../proto/micros/genai/v1/genai.proto"], &["../proto"])?;

    // Compile document proto (client only - for document fetching)
    tonic_build::configure()
        .build_server(false)
        .build_client(true)
        .compile_protos(
            &["../proto/micros/document/v1/document.proto"],
            &["../proto"],
        )?;

    println!("cargo:rerun-if-changed=../proto/micros/genai/v1/genai.proto");
    println!("cargo:rerun-if-changed=../proto/micros/document/v1/document.proto");

    Ok(())
}
