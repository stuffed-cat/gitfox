//! Health service implementation

use tonic::{Request, Response, Status};

use crate::proto::*;
use crate::proto::health_check_response::ServingStatus;

pub struct HealthServiceImpl;

impl HealthServiceImpl {
    pub fn new() -> Self {
        Self
    }
}

#[tonic::async_trait]
impl health_service_server::HealthService for HealthServiceImpl {
    async fn check(
        &self,
        _request: Request<HealthCheckRequest>,
    ) -> Result<Response<HealthCheckResponse>, Status> {
        Ok(Response::new(HealthCheckResponse {
            status: ServingStatus::Serving as i32,
            version: env!("CARGO_PKG_VERSION").to_string(),
            git2_version: git2::Version::get().crate_version().to_string(),
        }))
    }
}
