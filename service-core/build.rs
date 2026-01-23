use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let proto_root = PathBuf::from("../proto");

    // Tell cargo to recompile if any proto files change
    println!("cargo:rerun-if-changed=../proto/micros/auth/v1/");
    println!("cargo:rerun-if-changed=../proto/micros/notification/v1/");
    println!("cargo:rerun-if-changed=../proto/micros/document/v1/");
    println!("cargo:rerun-if-changed=../proto/micros/payment/v1/");
    println!("cargo:rerun-if-changed=../proto/micros/ledger/v1/");
    println!("cargo:rerun-if-changed=../proto/micros/common/");

    // Compile auth service protos (client-side)
    tonic_build::configure()
        .build_server(false) // No server code in service-core
        .build_client(true) // Build clients for calling auth-service
        .compile_protos(
            &[
                "../proto/micros/auth/v1/auth.proto",
                "../proto/micros/auth/v1/context.proto",
            ],
            &[&proto_root],
        )?;

    // Compile notification service protos (client-side)
    tonic_build::configure()
        .build_server(false) // No server code in service-core
        .build_client(true) // Build clients for calling notification-service
        .compile_protos(
            &[
                "../proto/micros/notification/v1/notification.proto",
                "../proto/micros/notification/v1/email.proto",
                "../proto/micros/notification/v1/sms.proto",
                "../proto/micros/notification/v1/push.proto",
            ],
            &[&proto_root],
        )?;

    // Compile document service protos (client-side)
    tonic_build::configure()
        .build_server(false) // No server code in service-core
        .build_client(true) // Build clients for calling document-service
        .compile_protos(
            &["../proto/micros/document/v1/document.proto"],
            &[&proto_root],
        )?;

    // Compile payment service protos (client-side)
    tonic_build::configure()
        .build_server(false) // No server code in service-core
        .build_client(true) // Build clients for calling payment-service
        .compile_protos(
            &[
                "../proto/micros/payment/v1/payment.proto",
                "../proto/micros/payment/v1/transaction.proto",
            ],
            &[&proto_root],
        )?;

    // Compile ledger service protos (client-side)
    tonic_build::configure()
        .build_server(false) // No server code in service-core
        .build_client(true) // Build clients for calling ledger-service
        .compile_protos(&["../proto/micros/ledger/v1/ledger.proto"], &[&proto_root])?;

    Ok(())
}
