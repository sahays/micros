fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = std::path::PathBuf::from(std::env::var("OUT_DIR")?);

    // Compile payment service protos with file descriptor set for reflection
    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .file_descriptor_set_path(out_dir.join("payment_descriptor.bin"))
        .compile_protos(
            &[
                "../proto/micros/payment/v1/payment.proto",
                "../proto/micros/payment/v1/transaction.proto",
            ],
            &["../proto"],
        )?;

    println!("cargo:rerun-if-changed=../proto/micros/payment/v1/payment.proto");
    println!("cargo:rerun-if-changed=../proto/micros/payment/v1/transaction.proto");

    Ok(())
}
