use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Webhook {
    pub id: Uuid,
    pub project_id: Uuid,
    pub url: String,
    pub secret: Option<String>,
    pub events: Vec<String>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateWebhookRequest {
    pub url: String,
    pub secret: Option<String>,
    pub events: Vec<WebhookEvent>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateWebhookRequest {
    pub url: Option<String>,
    pub secret: Option<String>,
    pub events: Option<Vec<WebhookEvent>>,
    pub is_active: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum WebhookEvent {
    Push,
    TagPush,
    MergeRequest,
    MergeRequestMerge,
    MergeRequestClose,
    PipelineSuccess,
    PipelineFailed,
    Release,
}

impl WebhookEvent {
    pub fn as_str(&self) -> &'static str {
        match self {
            WebhookEvent::Push => "push",
            WebhookEvent::TagPush => "tag_push",
            WebhookEvent::MergeRequest => "merge_request",
            WebhookEvent::MergeRequestMerge => "merge_request_merge",
            WebhookEvent::MergeRequestClose => "merge_request_close",
            WebhookEvent::PipelineSuccess => "pipeline_success",
            WebhookEvent::PipelineFailed => "pipeline_failed",
            WebhookEvent::Release => "release",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct WebhookDelivery {
    pub id: Uuid,
    pub webhook_id: Uuid,
    pub event: String,
    pub payload: serde_json::Value,
    pub response_status: Option<i32>,
    pub response_body: Option<String>,
    pub delivered_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct WebhookPayload {
    pub event: String,
    pub project_id: Uuid,
    pub project_name: String,
    pub timestamp: DateTime<Utc>,
    pub data: serde_json::Value,
}
