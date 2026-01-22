use actix_web::{web, HttpResponse};
use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::models::{CreateWebhookRequest, UpdateWebhookRequest, Webhook, WebhookDelivery, WebhookPayload};
use crate::queue::{messages::WebhookDeliveryMessage, RedisMessageQueue, QUEUE_WEBHOOK};
use crate::services::ProjectService;

pub async fn list_webhooks(
    pool: web::Data<PgPool>,
    path: web::Path<String>,
) -> AppResult<HttpResponse> {
    let project = ProjectService::get_project_by_slug(pool.get_ref(), &path.into_inner()).await?;
    
    let webhooks = sqlx::query_as::<_, Webhook>(
        "SELECT * FROM webhooks WHERE project_id = $1 ORDER BY created_at"
    )
    .bind(project.id)
    .fetch_all(pool.get_ref())
    .await?;
    
    Ok(HttpResponse::Ok().json(webhooks))
}

pub async fn create_webhook(
    pool: web::Data<PgPool>,
    path: web::Path<String>,
    body: web::Json<CreateWebhookRequest>,
) -> AppResult<HttpResponse> {
    let project = ProjectService::get_project_by_slug(pool.get_ref(), &path.into_inner()).await?;
    
    let id = Uuid::new_v4();
    let now = Utc::now();
    let events: Vec<String> = body.events.iter().map(|e| e.as_str().to_string()).collect();
    
    let webhook = sqlx::query_as::<_, Webhook>(
        r#"
        INSERT INTO webhooks (id, project_id, url, secret, events, is_active, created_at, updated_at)
        VALUES ($1, $2, $3, $4, $5, true, $6, $6)
        RETURNING *
        "#
    )
    .bind(id)
    .bind(project.id)
    .bind(&body.url)
    .bind(&body.secret)
    .bind(&events)
    .bind(now)
    .fetch_one(pool.get_ref())
    .await?;
    
    Ok(HttpResponse::Created().json(webhook))
}

pub async fn get_webhook(
    pool: web::Data<PgPool>,
    path: web::Path<(String, Uuid)>,
) -> AppResult<HttpResponse> {
    let (slug, webhook_id) = path.into_inner();
    let project = ProjectService::get_project_by_slug(pool.get_ref(), &slug).await?;
    
    let webhook = sqlx::query_as::<_, Webhook>(
        "SELECT * FROM webhooks WHERE id = $1 AND project_id = $2"
    )
    .bind(webhook_id)
    .bind(project.id)
    .fetch_optional(pool.get_ref())
    .await?
    .ok_or_else(|| AppError::NotFound("Webhook not found".to_string()))?;
    
    let deliveries = sqlx::query_as::<_, WebhookDelivery>(
        "SELECT * FROM webhook_deliveries WHERE webhook_id = $1 ORDER BY created_at DESC LIMIT 20"
    )
    .bind(webhook.id)
    .fetch_all(pool.get_ref())
    .await?;
    
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "webhook": webhook,
        "recent_deliveries": deliveries
    })))
}

pub async fn update_webhook(
    pool: web::Data<PgPool>,
    path: web::Path<(String, Uuid)>,
    body: web::Json<UpdateWebhookRequest>,
) -> AppResult<HttpResponse> {
    let (slug, webhook_id) = path.into_inner();
    let project = ProjectService::get_project_by_slug(pool.get_ref(), &slug).await?;
    
    let events: Option<Vec<String>> = body.events.as_ref().map(|e| {
        e.iter().map(|ev| ev.as_str().to_string()).collect()
    });
    
    let webhook = sqlx::query_as::<_, Webhook>(
        r#"
        UPDATE webhooks
        SET url = COALESCE($3, url),
            secret = COALESCE($4, secret),
            events = COALESCE($5, events),
            is_active = COALESCE($6, is_active),
            updated_at = NOW()
        WHERE id = $1 AND project_id = $2
        RETURNING *
        "#
    )
    .bind(webhook_id)
    .bind(project.id)
    .bind(&body.url)
    .bind(&body.secret)
    .bind(&events)
    .bind(body.is_active)
    .fetch_optional(pool.get_ref())
    .await?
    .ok_or_else(|| AppError::NotFound("Webhook not found".to_string()))?;
    
    Ok(HttpResponse::Ok().json(webhook))
}

pub async fn delete_webhook(
    pool: web::Data<PgPool>,
    path: web::Path<(String, Uuid)>,
) -> AppResult<HttpResponse> {
    let (slug, webhook_id) = path.into_inner();
    let project = ProjectService::get_project_by_slug(pool.get_ref(), &slug).await?;
    
    let result = sqlx::query("DELETE FROM webhooks WHERE id = $1 AND project_id = $2")
        .bind(webhook_id)
        .bind(project.id)
        .execute(pool.get_ref())
        .await?;
    
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("Webhook not found".to_string()));
    }
    
    Ok(HttpResponse::NoContent().finish())
}

pub async fn test_webhook(
    pool: web::Data<PgPool>,
    queue: web::Data<RedisMessageQueue>,
    path: web::Path<(String, Uuid)>,
) -> AppResult<HttpResponse> {
    let (slug, webhook_id) = path.into_inner();
    let project = ProjectService::get_project_by_slug(pool.get_ref(), &slug).await?;
    
    let webhook = sqlx::query_as::<_, Webhook>(
        "SELECT * FROM webhooks WHERE id = $1 AND project_id = $2"
    )
    .bind(webhook_id)
    .bind(project.id)
    .fetch_optional(pool.get_ref())
    .await?
    .ok_or_else(|| AppError::NotFound("Webhook not found".to_string()))?;
    
    // Create test delivery record
    let delivery_id = Uuid::new_v4();
    let now = Utc::now();
    
    let payload = WebhookPayload {
        event: "test".to_string(),
        project_id: project.id,
        project_name: project.name.clone(),
        timestamp: now,
        data: serde_json::json!({
            "message": "This is a test webhook delivery"
        }),
    };
    
    sqlx::query(
        r#"
        INSERT INTO webhook_deliveries (id, webhook_id, event, payload, created_at)
        VALUES ($1, $2, 'test', $3, $4)
        "#
    )
    .bind(delivery_id)
    .bind(webhook.id)
    .bind(serde_json::to_value(&payload).unwrap())
    .bind(now)
    .execute(pool.get_ref())
    .await?;
    
    // Queue delivery
    let message = WebhookDeliveryMessage {
        webhook_id: webhook.id,
        delivery_id,
        url: webhook.url.clone(),
        payload: serde_json::to_value(&payload).unwrap(),
        secret: webhook.secret.clone(),
    };
    queue.publish(QUEUE_WEBHOOK, &message).await?;
    
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "message": "Test webhook queued",
        "delivery_id": delivery_id
    })))
}
