#![allow(dead_code)]

pub fn grpc_endpoint() -> String {
    std::env::var("GENAI_GRPC_ENDPOINT").unwrap_or_else(|_| "http://localhost:50055".to_string())
}

pub fn http_base_url() -> String {
    std::env::var("GENAI_HTTP_URL").unwrap_or_else(|_| "http://localhost:9010".to_string())
}

/// Check if API tests should be skipped (no valid Google API key).
pub fn skip_api_tests() -> bool {
    std::env::var("SKIP_API_TESTS").is_ok()
        || std::env::var("GOOGLE_API_KEY")
            .map(|k| k.is_empty() || k == "test-api-key")
            .unwrap_or(true)
}
