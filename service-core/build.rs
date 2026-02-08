use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let proto_root = PathBuf::from("../proto");

    // Tell cargo to recompile if any proto files change
    println!("cargo:rerun-if-changed=../proto/micros/auth/v1/");
    println!("cargo:rerun-if-changed=../proto/micros/notification/v1/");
    println!("cargo:rerun-if-changed=../proto/micros/document/v1/");
    println!("cargo:rerun-if-changed=../proto/micros/payment/v1/");
    println!("cargo:rerun-if-changed=../proto/micros/ledger/v1/");
    println!("cargo:rerun-if-changed=../proto/micros/genai/v1/");
    println!("cargo:rerun-if-changed=../proto/micros/billing/v1/");
    println!("cargo:rerun-if-changed=../proto/micros/invoicing/v1/");
    println!("cargo:rerun-if-changed=../proto/micros/reconciliation/v1/");
    println!("cargo:rerun-if-changed=../proto/micros/common/");

    // Compile auth service protos (client-side)
    tonic_build::configure()
        .build_server(false) // No server code in service-core
        .build_client(true) // Build clients for calling auth-service
        .compile_protos(
            &[
                "../proto/micros/auth/v1/admin.proto",
                "../proto/micros/auth/v1/auth.proto",
                "../proto/micros/auth/v1/context.proto",
                "../proto/micros/auth/v1/org.proto",
                "../proto/micros/auth/v1/role.proto",
                "../proto/micros/auth/v1/assignment.proto",
                "../proto/micros/auth/v1/invitation.proto",
                "../proto/micros/auth/v1/visibility.proto",
                "../proto/micros/auth/v1/audit.proto",
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
                "../proto/micros/payment/v1/linked_account.proto",
                "../proto/micros/payment/v1/customer.proto",
                "../proto/micros/payment/v1/transfer.proto",
                "../proto/micros/payment/v1/settlement.proto",
                "../proto/micros/payment/v1/subscription.proto",
                "../proto/micros/payment/v1/payment_link.proto",
                "../proto/micros/payment/v1/refund.proto",
            ],
            &[&proto_root],
        )?;

    // Compile ledger service protos (client-side)
    tonic_build::configure()
        .build_server(false) // No server code in service-core
        .build_client(true) // Build clients for calling ledger-service
        .compile_protos(&["../proto/micros/ledger/v1/ledger.proto"], &[&proto_root])?;

    // Compile genai service protos (client-side)
    tonic_build::configure()
        .build_server(false) // No server code in service-core
        .build_client(true) // Build clients for calling genai-service
        .compile_protos(&["../proto/micros/genai/v1/genai.proto"], &[&proto_root])?;

    // Compile billing service protos (client-side)
    tonic_build::configure()
        .build_server(false)
        .build_client(true)
        .compile_protos(
            &["../proto/micros/billing/v1/billing.proto"],
            &[&proto_root],
        )?;

    // Compile invoicing service protos (client-side)
    tonic_build::configure()
        .build_server(false)
        .build_client(true)
        .compile_protos(
            &["../proto/micros/invoicing/v1/invoicing.proto"],
            &[&proto_root],
        )?;

    // Compile reconciliation service protos (client-side)
    tonic_build::configure()
        .build_server(false)
        .build_client(true)
        .compile_protos(
            &["../proto/micros/reconciliation/v1/reconciliation.proto"],
            &[&proto_root],
        )?;

    Ok(())
}
