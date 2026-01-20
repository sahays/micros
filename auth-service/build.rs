use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let proto_root = PathBuf::from("../proto");

    // Tell cargo to recompile if any proto files change
    println!("cargo:rerun-if-changed=../proto/micros/auth/v1/");
    println!("cargo:rerun-if-changed=../proto/micros/common/");

    // Compile auth service protos
    tonic_build::configure()
        .build_server(true)
        .build_client(false) // Clients will be built in service-core
        .file_descriptor_set_path(
            PathBuf::from(std::env::var("OUT_DIR")?).join("auth_service_descriptor.bin"),
        )
        .compile_protos(
            &[
                "../proto/micros/auth/v1/auth.proto",
                "../proto/micros/auth/v1/context.proto",
                "../proto/micros/auth/v1/org.proto",
                "../proto/micros/auth/v1/role.proto",
                "../proto/micros/auth/v1/assignment.proto",
                "../proto/micros/auth/v1/invitation.proto",
                "../proto/micros/auth/v1/visibility.proto",
                "../proto/micros/auth/v1/audit.proto",
                "../proto/micros/auth/v1/service_registry.proto",
            ],
            &[proto_root],
        )?;

    Ok(())
}
