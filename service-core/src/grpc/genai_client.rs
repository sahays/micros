//! GenAI service gRPC client for service-to-service communication.
//!
//! Provides a high-level client for calling genai-service with built-in retry support.

use std::time::Duration;
use tonic::Request;
use tonic::transport::{Channel, Endpoint};

use super::proto::genai::gen_ai_service_client::GenAiServiceClient;
use super::proto::genai::{
    DocumentContext, GenerationParams, OutputFormat, ProcessRequest, ProcessResponse,
    RequestMetadata,
};
use super::retry::{RetryConfig, retry_grpc_call};

/// Configuration for the genai service client.
#[derive(Clone, Debug)]
pub struct GenaiClientConfig {
    /// The gRPC endpoint of the genai service.
    pub endpoint: String,
    /// Connection timeout.
    pub connect_timeout: Duration,
    /// Request timeout.
    pub request_timeout: Duration,
    /// Retry configuration.
    pub retry_config: RetryConfig,
}

impl Default for GenaiClientConfig {
    fn default() -> Self {
        Self {
            endpoint: "http://localhost:50054".to_string(),
            connect_timeout: Duration::from_secs(5),
            request_timeout: Duration::from_secs(120), // AI processing can be slow
            retry_config: RetryConfig::default(),
        }
    }
}

/// GenAI service client with retry support.
#[derive(Clone)]
pub struct GenaiClient {
    client: GenAiServiceClient<Channel>,
    retry_config: RetryConfig,
}

impl GenaiClient {
    /// Create a new genai client with the given configuration.
    pub async fn new(config: GenaiClientConfig) -> Result<Self, tonic::transport::Error> {
        let channel = Endpoint::from_shared(config.endpoint)?
            .connect_timeout(config.connect_timeout)
            .timeout(config.request_timeout)
            .connect()
            .await?;

        Ok(Self {
            client: GenAiServiceClient::new(channel),
            retry_config: config.retry_config,
        })
    }

    /// Create a new genai client connecting to the specified endpoint.
    pub async fn connect(endpoint: &str) -> Result<Self, tonic::transport::Error> {
        Self::new(GenaiClientConfig {
            endpoint: endpoint.to_string(),
            ..Default::default()
        })
        .await
    }

    /// Create a new genai client with custom retry configuration.
    pub async fn with_retry(
        endpoint: &str,
        retry_config: RetryConfig,
    ) -> Result<Self, tonic::transport::Error> {
        Self::new(GenaiClientConfig {
            endpoint: endpoint.to_string(),
            retry_config,
            ..Default::default()
        })
        .await
    }

    /// Process a prompt and get structured JSON output.
    ///
    /// This is the primary method for document extraction tasks like bank statement parsing.
    pub async fn process_structured(
        &self,
        prompt: &str,
        documents: Vec<DocumentContext>,
        output_schema: &str,
        tenant_id: &str,
        user_id: &str,
    ) -> Result<ProcessResponse, tonic::Status> {
        let client = self.client.clone();
        let request = ProcessRequest {
            prompt: prompt.to_string(),
            documents,
            output_format: OutputFormat::StructuredJson.into(),
            output_schema: Some(output_schema.to_string()),
            session_id: None,
            params: Some(GenerationParams {
                temperature: Some(0.0), // Deterministic output for parsing
                top_p: None,
                stop_sequences: vec![],
                content_size_threshold_bytes: None,
                voice: None,
                audio_format: None,
                video_format: None,
                duration_seconds: None,
            }),
            metadata: Some(RequestMetadata {
                tenant_id: tenant_id.to_string(),
                user_id: user_id.to_string(),
                tags: std::collections::HashMap::new(),
            }),
        };

        retry_grpc_call(&self.retry_config, "process_structured", || {
            let mut c = client.clone();
            let req = request.clone();
            async move {
                let response = c.process(Request::new(req)).await?;
                Ok(response.into_inner())
            }
        })
        .await
    }

    /// Process a prompt and get text output.
    pub async fn process_text(
        &self,
        prompt: &str,
        documents: Vec<DocumentContext>,
        tenant_id: &str,
        user_id: &str,
    ) -> Result<ProcessResponse, tonic::Status> {
        let client = self.client.clone();
        let request = ProcessRequest {
            prompt: prompt.to_string(),
            documents,
            output_format: OutputFormat::Text.into(),
            output_schema: None,
            session_id: None,
            params: None,
            metadata: Some(RequestMetadata {
                tenant_id: tenant_id.to_string(),
                user_id: user_id.to_string(),
                tags: std::collections::HashMap::new(),
            }),
        };

        retry_grpc_call(&self.retry_config, "process_text", || {
            let mut c = client.clone();
            let req = request.clone();
            async move {
                let response = c.process(Request::new(req)).await?;
                Ok(response.into_inner())
            }
        })
        .await
    }
}

/// Bank statement extraction schema (v1).
/// Use this with `process_structured` for bank statement parsing.
pub const BANK_STATEMENT_SCHEMA_V1: &str = r#"{
  "type": "object",
  "properties": {
    "statement": {
      "type": "object",
      "properties": {
        "period_start": {"type": "string", "format": "date"},
        "period_end": {"type": "string", "format": "date"},
        "opening_balance": {
          "type": "object",
          "properties": {
            "value": {"type": "string"},
            "confidence": {"type": "number", "minimum": 0, "maximum": 1}
          },
          "required": ["value", "confidence"]
        },
        "closing_balance": {
          "type": "object",
          "properties": {
            "value": {"type": "string"},
            "confidence": {"type": "number", "minimum": 0, "maximum": 1}
          },
          "required": ["value", "confidence"]
        }
      },
      "required": ["period_start", "period_end", "opening_balance", "closing_balance"]
    },
    "transactions": {
      "type": "array",
      "items": {
        "type": "object",
        "properties": {
          "date": {
            "type": "object",
            "properties": {
              "value": {"type": "string", "format": "date"},
              "confidence": {"type": "number", "minimum": 0, "maximum": 1}
            },
            "required": ["value", "confidence"]
          },
          "description": {
            "type": "object",
            "properties": {
              "value": {"type": "string"},
              "confidence": {"type": "number", "minimum": 0, "maximum": 1}
            },
            "required": ["value", "confidence"]
          },
          "reference": {
            "type": "object",
            "properties": {
              "value": {"type": "string"},
              "confidence": {"type": "number", "minimum": 0, "maximum": 1}
            }
          },
          "amount": {
            "type": "object",
            "properties": {
              "value": {"type": "string"},
              "confidence": {"type": "number", "minimum": 0, "maximum": 1}
            },
            "required": ["value", "confidence"]
          },
          "running_balance": {
            "type": "object",
            "properties": {
              "value": {"type": "string"},
              "confidence": {"type": "number", "minimum": 0, "maximum": 1}
            }
          }
        },
        "required": ["date", "description", "amount"]
      }
    },
    "extraction_metadata": {
      "type": "object",
      "properties": {
        "overall_confidence": {"type": "number", "minimum": 0, "maximum": 1},
        "page_count": {"type": "integer"},
        "warnings": {"type": "array", "items": {"type": "string"}}
      }
    }
  },
  "required": ["statement", "transactions"]
}"#;

/// Bank statement extraction prompt template.
pub const BANK_STATEMENT_PROMPT: &str = r#"Extract all transactions from this bank statement.

For each transaction, extract:
- Date (transaction date)
- Description (full transaction description)
- Reference (if present, extract reference/check number)
- Amount (positive for deposits/credits, negative for withdrawals/debits)
- Running balance (if shown)

Also extract:
- Statement period (start and end dates)
- Opening balance
- Closing balance

Provide a confidence score (0-1) for each extracted field.
If any field is unclear or partially visible, use a lower confidence score."#;

// Re-export useful types from proto
pub use super::proto::genai::{
    DocumentContext as DocumentContextProto, OutputFormat as OutputFormatProto,
    ProcessResponse as ProcessResponseProto,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_genai_client_config_default() {
        let config = GenaiClientConfig::default();
        assert_eq!(config.endpoint, "http://localhost:50054");
        assert_eq!(config.connect_timeout, Duration::from_secs(5));
        assert_eq!(config.request_timeout, Duration::from_secs(120));
    }

    #[test]
    fn test_bank_statement_schema_is_valid_json() {
        let _: serde_json::Value =
            serde_json::from_str(BANK_STATEMENT_SCHEMA_V1).expect("Schema should be valid JSON");
    }
}
