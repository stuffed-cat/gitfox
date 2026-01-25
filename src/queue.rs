use deadpool_redis::Pool as RedisPool;
use redis::AsyncCommands;
use serde::{de::DeserializeOwned, Serialize};
use std::time::Duration;

use crate::error::{AppError, AppResult};

#[derive(Clone)]
pub struct RedisMessageQueue {
    pool: RedisPool,
}

impl RedisMessageQueue {
    pub fn new(pool: RedisPool) -> Self {
        Self { pool }
    }

    /// Publish a message to a queue
    pub async fn publish<T: Serialize>(&self, queue: &str, message: &T) -> AppResult<()> {
        let mut conn = self.pool.get().await.map_err(|e| {
            AppError::QueueError(format!("Failed to get Redis connection: {}", e))
        })?;

        let payload = serde_json::to_string(message).map_err(|e| {
            AppError::QueueError(format!("Failed to serialize message: {}", e))
        })?;

        conn.lpush::<_, _, ()>(queue, &payload).await?;
        Ok(())
    }

    /// Subscribe and receive a message from a queue (blocking)
    pub async fn subscribe<T: DeserializeOwned>(&self, queue: &str, timeout: Duration) -> AppResult<Option<T>> {
        let mut conn = self.pool.get().await.map_err(|e| {
            AppError::QueueError(format!("Failed to get Redis connection: {}", e))
        })?;

        let result: Option<(String, String)> = conn
            .brpop(queue, timeout.as_secs() as f64)
            .await?;

        match result {
            Some((_, payload)) => {
                let message: T = serde_json::from_str(&payload).map_err(|e| {
                    AppError::QueueError(format!("Failed to deserialize message: {}", e))
                })?;
                Ok(Some(message))
            }
            None => Ok(None),
        }
    }

    /// Get the length of a queue
    pub async fn queue_length(&self, queue: &str) -> AppResult<usize> {
        let mut conn = self.pool.get().await.map_err(|e| {
            AppError::QueueError(format!("Failed to get Redis connection: {}", e))
        })?;

        let len: usize = conn.llen(queue).await?;
        Ok(len)
    }

    /// Clear a queue
    pub async fn clear_queue(&self, queue: &str) -> AppResult<()> {
        let mut conn = self.pool.get().await.map_err(|e| {
            AppError::QueueError(format!("Failed to get Redis connection: {}", e))
        })?;

        conn.del::<_, ()>(queue).await?;
        Ok(())
    }

    /// Publish with delay (using sorted set)
    pub async fn publish_delayed<T: Serialize>(
        &self,
        queue: &str,
        message: &T,
        delay: Duration,
    ) -> AppResult<()> {
        let mut conn = self.pool.get().await.map_err(|e| {
            AppError::QueueError(format!("Failed to get Redis connection: {}", e))
        })?;

        let payload = serde_json::to_string(message).map_err(|e| {
            AppError::QueueError(format!("Failed to serialize message: {}", e))
        })?;

        let delayed_queue = format!("{}:delayed", queue);
        let score = chrono::Utc::now().timestamp() as f64 + delay.as_secs_f64();

        conn.zadd::<_, _, _, ()>(&delayed_queue, &payload, score).await?;
        Ok(())
    }

    /// Process delayed messages (move ready messages to the main queue)
    pub async fn process_delayed(&self, queue: &str) -> AppResult<usize> {
        let mut conn = self.pool.get().await.map_err(|e| {
            AppError::QueueError(format!("Failed to get Redis connection: {}", e))
        })?;

        let delayed_queue = format!("{}:delayed", queue);
        let now = chrono::Utc::now().timestamp() as f64;

        let messages: Vec<String> = conn
            .zrangebyscore(&delayed_queue, 0.0, now)
            .await?;

        let count = messages.len();

        for msg in messages {
            conn.zrem::<_, _, ()>(&delayed_queue, &msg).await?;
            conn.lpush::<_, _, ()>(queue, &msg).await?;
        }

        Ok(count)
    }
}

// Message types for different operations
pub mod messages {
    use serde::{Deserialize, Serialize};
    

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct PipelineTriggerMessage {
        pub pipeline_id: i64,
        pub project_id: i64,
        pub ref_name: String,
        pub commit_sha: String,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct WebhookDeliveryMessage {
        pub webhook_id: i64,
        pub delivery_id: i64,
        pub url: String,
        pub payload: serde_json::Value,
        pub secret: Option<String>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct GitOperationMessage {
        pub operation: GitOperation,
        pub project_id: i64,
        pub user_id: i64,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub enum GitOperation {
        Clone { url: String },
        Push { ref_name: String },
        Merge { source: String, target: String },
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct NotificationMessage {
        pub user_id: i64,
        pub notification_type: NotificationType,
        pub title: String,
        pub content: String,
        pub link: Option<String>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub enum NotificationType {
        MergeRequestCreated,
        MergeRequestMerged,
        MergeRequestCommented,
        PipelineCompleted,
        PipelineFailed,
        ReviewRequested,
    }
}

// Queue names
pub const QUEUE_PIPELINE: &str = "devops:queue:pipeline";
pub const QUEUE_WEBHOOK: &str = "devops:queue:webhook";
pub const QUEUE_GIT_OPERATION: &str = "devops:queue:git";
pub const QUEUE_NOTIFICATION: &str = "devops:queue:notification";
