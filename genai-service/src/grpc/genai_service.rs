//! GenAI gRPC service — slim delegation layer.

use crate::grpc::proto::{
    gen_ai_service_server::GenAiService, CreateSessionRequest, CreateSessionResponse,
    DeleteSessionRequest, DeleteSessionResponse, GetSessionRequest, GetSessionResponse,
    GetUsageRequest, GetUsageResponse, ListModelsRequest, ListModelsResponse, ProcessRequest,
    ProcessResponse, ProcessStreamRequest, ProcessStreamResponse,
};
use crate::startup::AppState;
use futures::Stream;
use std::pin::Pin;
use tonic::{Request, Response, Status};

use super::{processing, sessions};

pub type ProcessStreamResult =
    Pin<Box<dyn Stream<Item = Result<ProcessStreamResponse, Status>> + Send>>;

pub struct GenaiGrpcService {
    state: AppState,
}

impl GenaiGrpcService {
    pub fn new(state: AppState) -> Self {
        Self { state }
    }
}

#[tonic::async_trait]
impl GenAiService for GenaiGrpcService {
    type ProcessStreamStream = ProcessStreamResult;

    async fn process(
        &self,
        request: Request<ProcessRequest>,
    ) -> Result<Response<ProcessResponse>, Status> {
        processing::process(&self.state, request).await
    }

    async fn process_stream(
        &self,
        request: Request<ProcessStreamRequest>,
    ) -> Result<Response<Self::ProcessStreamStream>, Status> {
        processing::process_stream(&self.state, request).await
    }

    async fn create_session(
        &self,
        request: Request<CreateSessionRequest>,
    ) -> Result<Response<CreateSessionResponse>, Status> {
        sessions::create_session(&self.state, request).await
    }

    async fn get_session(
        &self,
        request: Request<GetSessionRequest>,
    ) -> Result<Response<GetSessionResponse>, Status> {
        sessions::get_session(&self.state, request).await
    }

    async fn delete_session(
        &self,
        request: Request<DeleteSessionRequest>,
    ) -> Result<Response<DeleteSessionResponse>, Status> {
        sessions::delete_session(&self.state, request).await
    }

    async fn get_usage(
        &self,
        request: Request<GetUsageRequest>,
    ) -> Result<Response<GetUsageResponse>, Status> {
        sessions::get_usage(&self.state, request).await
    }

    async fn list_models(
        &self,
        request: Request<ListModelsRequest>,
    ) -> Result<Response<ListModelsResponse>, Status> {
        sessions::list_models(&self.state, request).await
    }
}
