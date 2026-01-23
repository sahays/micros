//! gRPC integration tests for genai-service.
//!
//! These tests verify the gRPC endpoints work correctly.
//! Tests that require a valid GOOGLE_API_KEY will be skipped if SKIP_API_TESTS is set.
//! Run with: cargo test -p genai-service --test grpc_test

use genai_service::config::GenaiConfig;
use genai_service::grpc::proto::{
    gen_ai_service_client::GenAiServiceClient, ListModelsRequest, OutputFormat, ProcessRequest,
    RequestMetadata,
};
use genai_service::startup::Application;
use std::time::Duration;
use tonic::transport::Channel;

/// Check if API tests should be skipped.
fn skip_api_tests() -> bool {
    std::env::var("SKIP_API_TESTS").is_ok()
        || std::env::var("GOOGLE_API_KEY")
            .map(|k| k.is_empty() || k == "test-api-key")
            .unwrap_or(true)
}

/// Spawn the application and return the gRPC port.
async fn spawn_app() -> u16 {
    // Set test environment variables
    std::env::set_var("ENVIRONMENT", "test");
    std::env::set_var("APP__PORT", "0");
    std::env::set_var("MONGODB_URI", "mongodb://localhost:27017");
    std::env::set_var("MONGODB_DATABASE", "genai_test_db");
    std::env::set_var("GOOGLE_API_KEY", "test-api-key");
    std::env::set_var("GENAI_TEXT_MODEL", "gemini-2.0-flash");
    std::env::set_var("GENAI_AUDIO_MODEL", "gemini-2.0-flash");
    std::env::set_var("GENAI_VIDEO_MODEL", "veo-2");

    let config = GenaiConfig::load().expect("Failed to load config");
    let app = Application::build(config)
        .await
        .expect("Failed to build application");

    let grpc_port = app.grpc_port();

    // Spawn the server in the background
    tokio::spawn(async move {
        let _ = app.run_until_stopped().await;
    });

    // Wait for server to start
    tokio::time::sleep(Duration::from_millis(200)).await;

    grpc_port
}

async fn create_client(port: u16) -> GenAiServiceClient<Channel> {
    let addr = format!("http://localhost:{}", port);

    // Retry connection a few times
    for _ in 0..5 {
        match GenAiServiceClient::connect(addr.clone()).await {
            Ok(client) => return client,
            Err(_) => {
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        }
    }

    GenAiServiceClient::connect(addr)
        .await
        .expect("Failed to connect to gRPC server")
}

#[tokio::test]
async fn list_models_returns_configured_models() {
    // Skip if MongoDB is not available
    if std::env::var("SKIP_MONGO_TESTS").is_ok() {
        eprintln!("Skipping test: SKIP_MONGO_TESTS is set");
        return;
    }

    let port = spawn_app().await;
    let mut client = create_client(port).await;

    let response = client
        .list_models(ListModelsRequest {})
        .await
        .expect("Failed to list models");

    let models = response.into_inner().models;
    assert_eq!(models.len(), 3);

    // Check that we have text, audio, and video models
    let model_ids: Vec<&str> = models.iter().map(|m| m.id.as_str()).collect();
    assert!(model_ids.contains(&"gemini-2.0-flash")); // Text model
    assert!(model_ids.contains(&"veo-2")); // Video model
}

#[tokio::test]
async fn process_returns_text_response() {
    // Skip if MongoDB is not available
    if std::env::var("SKIP_MONGO_TESTS").is_ok() {
        eprintln!("Skipping test: SKIP_MONGO_TESTS is set");
        return;
    }

    // Skip if no valid API key is available
    if skip_api_tests() {
        eprintln!("Skipping test: SKIP_API_TESTS is set or GOOGLE_API_KEY is not valid");
        return;
    }

    let port = spawn_app().await;
    let mut client = create_client(port).await;

    let response = client
        .process(ProcessRequest {
            prompt: "Say 'Hello' in exactly one word.".to_string(),
            documents: vec![],
            output_format: OutputFormat::Text as i32,
            output_schema: None,
            session_id: None,
            params: None,
            metadata: Some(RequestMetadata {
                tenant_id: "test-tenant".to_string(),
                user_id: "test-user".to_string(),
                tags: Default::default(),
            }),
        })
        .await
        .expect("Failed to process request");

    let response = response.into_inner();

    // Check that we get a valid response
    assert!(!response.request_id.is_empty());
    assert_eq!(response.output_format, OutputFormat::Text as i32);
    assert!(!response.model.is_empty());

    // Should have token usage
    assert!(response.usage.is_some());
    let usage = response.usage.unwrap();
    assert!(usage.input_tokens > 0);
    assert!(usage.output_tokens > 0);

    // Should have a text response
    match response.result {
        Some(genai_service::grpc::proto::process_response::Result::Text(text)) => {
            assert!(!text.is_empty());
        }
        _ => panic!("Expected text response"),
    }
}

#[tokio::test]
async fn process_returns_error_for_invalid_api_key() {
    // Skip if MongoDB is not available
    if std::env::var("SKIP_MONGO_TESTS").is_ok() {
        eprintln!("Skipping test: SKIP_MONGO_TESTS is set");
        return;
    }

    // Only run this test when we're using a test API key (invalid)
    if !skip_api_tests() {
        eprintln!("Skipping test: valid GOOGLE_API_KEY is set");
        return;
    }

    let port = spawn_app().await;
    let mut client = create_client(port).await;

    let result = client
        .process(ProcessRequest {
            prompt: "Hello, world!".to_string(),
            documents: vec![],
            output_format: OutputFormat::Text as i32,
            output_schema: None,
            session_id: None,
            params: None,
            metadata: Some(RequestMetadata {
                tenant_id: "test-tenant".to_string(),
                user_id: "test-user".to_string(),
                tags: Default::default(),
            }),
        })
        .await;

    // Should return an error for invalid API key
    assert!(result.is_err());
    let status = result.unwrap_err();
    // Internal error from provider API error
    assert_eq!(status.code(), tonic::Code::Internal);
    assert!(
        status.message().contains("API key") || status.message().contains("Provider API error")
    );
}

#[tokio::test]
async fn process_rejects_empty_prompt() {
    // Skip if MongoDB is not available
    if std::env::var("SKIP_MONGO_TESTS").is_ok() {
        eprintln!("Skipping test: SKIP_MONGO_TESTS is set");
        return;
    }

    let port = spawn_app().await;
    let mut client = create_client(port).await;

    let result = client
        .process(ProcessRequest {
            prompt: "".to_string(), // Empty prompt
            documents: vec![],
            output_format: OutputFormat::Text as i32,
            output_schema: None,
            session_id: None,
            params: None,
            metadata: Some(RequestMetadata {
                tenant_id: "test-tenant".to_string(),
                user_id: "test-user".to_string(),
                tags: Default::default(),
            }),
        })
        .await;

    // Should return an error for empty prompt
    assert!(result.is_err());
    let status = result.unwrap_err();
    assert_eq!(status.code(), tonic::Code::InvalidArgument);
}

#[tokio::test]
async fn process_requires_schema_for_structured_json() {
    // Skip if MongoDB is not available
    if std::env::var("SKIP_MONGO_TESTS").is_ok() {
        eprintln!("Skipping test: SKIP_MONGO_TESTS is set");
        return;
    }

    let port = spawn_app().await;
    let mut client = create_client(port).await;

    let result = client
        .process(ProcessRequest {
            prompt: "Extract data".to_string(),
            documents: vec![],
            output_format: OutputFormat::StructuredJson as i32,
            output_schema: None, // Missing schema
            session_id: None,
            params: None,
            metadata: Some(RequestMetadata {
                tenant_id: "test-tenant".to_string(),
                user_id: "test-user".to_string(),
                tags: Default::default(),
            }),
        })
        .await;

    // Should return an error for missing schema
    assert!(result.is_err());
    let status = result.unwrap_err();
    assert_eq!(status.code(), tonic::Code::InvalidArgument);
    assert!(status.message().contains("output_schema"));
}

#[tokio::test]
async fn process_rejects_invalid_json_schema() {
    // Skip if MongoDB is not available
    if std::env::var("SKIP_MONGO_TESTS").is_ok() {
        eprintln!("Skipping test: SKIP_MONGO_TESTS is set");
        return;
    }

    let port = spawn_app().await;
    let mut client = create_client(port).await;

    let result = client
        .process(ProcessRequest {
            prompt: "Extract data".to_string(),
            documents: vec![],
            output_format: OutputFormat::StructuredJson as i32,
            output_schema: Some("not valid json".to_string()), // Invalid JSON
            session_id: None,
            params: None,
            metadata: Some(RequestMetadata {
                tenant_id: "test-tenant".to_string(),
                user_id: "test-user".to_string(),
                tags: Default::default(),
            }),
        })
        .await;

    // Should return an error for invalid JSON schema
    assert!(result.is_err());
    let status = result.unwrap_err();
    assert_eq!(status.code(), tonic::Code::InvalidArgument);
    assert!(status.message().contains("valid JSON"));
}

#[tokio::test]
async fn session_lifecycle() {
    use genai_service::grpc::proto::{
        CreateSessionRequest, DeleteSessionRequest, GetSessionRequest,
    };

    // Skip if MongoDB is not available
    if std::env::var("SKIP_MONGO_TESTS").is_ok() {
        eprintln!("Skipping test: SKIP_MONGO_TESTS is set");
        return;
    }

    let port = spawn_app().await;
    let mut client = create_client(port).await;

    // Create a session
    let create_response = client
        .create_session(CreateSessionRequest {
            title: Some("Test Session".to_string()),
            system_prompt: Some("You are a helpful assistant.".to_string()),
            documents: vec![],
            metadata: Some(RequestMetadata {
                tenant_id: "test-tenant".to_string(),
                user_id: "test-user".to_string(),
                tags: Default::default(),
            }),
        })
        .await
        .expect("Failed to create session");

    let session = create_response
        .into_inner()
        .session
        .expect("Session should be returned");
    assert!(!session.id.is_empty());
    assert_eq!(session.title, Some("Test Session".to_string()));
    assert_eq!(session.message_count, 0);

    let session_id = session.id.clone();

    // Get the session
    let get_response = client
        .get_session(GetSessionRequest {
            session_id: session_id.clone(),
            include_messages: true,
        })
        .await
        .expect("Failed to get session");

    let retrieved_session = get_response
        .into_inner()
        .session
        .expect("Session should be returned");
    assert_eq!(retrieved_session.id, session_id);
    assert_eq!(retrieved_session.title, Some("Test Session".to_string()));

    // Delete the session
    let delete_response = client
        .delete_session(DeleteSessionRequest {
            session_id: session_id.clone(),
        })
        .await
        .expect("Failed to delete session");

    assert!(delete_response.into_inner().success);

    // Verify session is deleted
    let get_result = client
        .get_session(GetSessionRequest {
            session_id: session_id.clone(),
            include_messages: false,
        })
        .await;

    assert!(get_result.is_err());
    assert_eq!(get_result.unwrap_err().code(), tonic::Code::NotFound);
}

#[tokio::test]
async fn create_session_requires_metadata() {
    use genai_service::grpc::proto::CreateSessionRequest;

    // Skip if MongoDB is not available
    if std::env::var("SKIP_MONGO_TESTS").is_ok() {
        eprintln!("Skipping test: SKIP_MONGO_TESTS is set");
        return;
    }

    let port = spawn_app().await;
    let mut client = create_client(port).await;

    let result = client
        .create_session(CreateSessionRequest {
            title: Some("Test Session".to_string()),
            system_prompt: None,
            documents: vec![],
            metadata: None, // Missing metadata
        })
        .await;

    assert!(result.is_err());
    let status = result.unwrap_err();
    assert_eq!(status.code(), tonic::Code::InvalidArgument);
    assert!(status.message().contains("metadata"));
}

#[tokio::test]
async fn get_session_requires_session_id() {
    use genai_service::grpc::proto::GetSessionRequest;

    // Skip if MongoDB is not available
    if std::env::var("SKIP_MONGO_TESTS").is_ok() {
        eprintln!("Skipping test: SKIP_MONGO_TESTS is set");
        return;
    }

    let port = spawn_app().await;
    let mut client = create_client(port).await;

    let result = client
        .get_session(GetSessionRequest {
            session_id: "".to_string(), // Empty session ID
            include_messages: false,
        })
        .await;

    assert!(result.is_err());
    let status = result.unwrap_err();
    assert_eq!(status.code(), tonic::Code::InvalidArgument);
}
