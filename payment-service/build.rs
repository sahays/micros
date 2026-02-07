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
                "../proto/micros/payment/v1/linked_account.proto",
                "../proto/micros/payment/v1/customer.proto",
                "../proto/micros/payment/v1/transfer.proto",
                "../proto/micros/payment/v1/settlement.proto",
                "../proto/micros/payment/v1/subscription.proto",
                "../proto/micros/payment/v1/payment_link.proto",
                "../proto/micros/payment/v1/refund.proto",
            ],
            &["../proto"],
        )?;

    println!("cargo:rerun-if-changed=../proto/micros/payment/v1/payment.proto");
    println!("cargo:rerun-if-changed=../proto/micros/payment/v1/transaction.proto");
    println!("cargo:rerun-if-changed=../proto/micros/payment/v1/linked_account.proto");
    println!("cargo:rerun-if-changed=../proto/micros/payment/v1/customer.proto");
    println!("cargo:rerun-if-changed=../proto/micros/payment/v1/transfer.proto");
    println!("cargo:rerun-if-changed=../proto/micros/payment/v1/settlement.proto");
    println!("cargo:rerun-if-changed=../proto/micros/payment/v1/subscription.proto");
    println!("cargo:rerun-if-changed=../proto/micros/payment/v1/payment_link.proto");
    println!("cargo:rerun-if-changed=../proto/micros/payment/v1/refund.proto");

    Ok(())
}
