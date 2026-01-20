fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = std::path::PathBuf::from(std::env::var("OUT_DIR")?);

    // Compile notification service protos with file descriptor set for reflection
    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .file_descriptor_set_path(out_dir.join("notification_descriptor.bin"))
        .compile_protos(
            &[
                "../proto/micros/notification/v1/notification.proto",
                "../proto/micros/notification/v1/email.proto",
                "../proto/micros/notification/v1/sms.proto",
                "../proto/micros/notification/v1/push.proto",
            ],
            &["../proto"],
        )?;

    println!("cargo:rerun-if-changed=../proto/micros/notification/v1/notification.proto");
    println!("cargo:rerun-if-changed=../proto/micros/notification/v1/email.proto");
    println!("cargo:rerun-if-changed=../proto/micros/notification/v1/sms.proto");
    println!("cargo:rerun-if-changed=../proto/micros/notification/v1/push.proto");

    Ok(())
}
