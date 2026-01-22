pub mod auth;
pub mod user;
pub mod project;
pub mod repository;
pub mod branch;
pub mod commit;
pub mod tag;
pub mod merge_request;
pub mod pipeline;
pub mod webhook;

use actix_web::web;

pub fn configure_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api/v1")
            // Auth routes
            .route("/auth/register", web::post().to(auth::register))
            .route("/auth/login", web::post().to(auth::login))
            .route("/auth/me", web::get().to(auth::me))
            
            // User routes
            .route("/users", web::get().to(user::list_users))
            .route("/users/{id}", web::get().to(user::get_user))
            .route("/users/{id}", web::put().to(user::update_user))
            .route("/users/{id}", web::delete().to(user::delete_user))
            
            // Project routes
            .route("/projects", web::get().to(project::list_projects))
            .route("/projects", web::post().to(project::create_project))
            .route("/projects/{slug}", web::get().to(project::get_project))
            .route("/projects/{slug}", web::put().to(project::update_project))
            .route("/projects/{slug}", web::delete().to(project::delete_project))
            .route("/projects/{slug}/stats", web::get().to(project::get_project_stats))
            .route("/projects/{slug}/members", web::get().to(project::get_members))
            .route("/projects/{slug}/members", web::post().to(project::add_member))
            .route("/projects/{slug}/members/{user_id}", web::delete().to(project::remove_member))
            
            // Repository routes
            .route("/projects/{slug}/repository", web::get().to(repository::get_repository_info))
            .route("/projects/{slug}/repository/tree", web::get().to(repository::browse_tree))
            .route("/projects/{slug}/repository/files", web::get().to(repository::get_file))
            
            // Branch routes
            .route("/projects/{slug}/branches", web::get().to(branch::list_branches))
            .route("/projects/{slug}/branches", web::post().to(branch::create_branch))
            .route("/projects/{slug}/branches/{name}", web::get().to(branch::get_branch))
            .route("/projects/{slug}/branches/{name}", web::delete().to(branch::delete_branch))
            
            // Commit routes
            .route("/projects/{slug}/commits", web::get().to(commit::list_commits))
            .route("/projects/{slug}/commits/{sha}", web::get().to(commit::get_commit))
            .route("/projects/{slug}/compare", web::get().to(commit::compare))
            
            // Tag routes
            .route("/projects/{slug}/tags", web::get().to(tag::list_tags))
            .route("/projects/{slug}/tags", web::post().to(tag::create_tag))
            .route("/projects/{slug}/tags/{name}", web::get().to(tag::get_tag))
            .route("/projects/{slug}/tags/{name}", web::delete().to(tag::delete_tag))
            
            // Merge Request routes
            .route("/projects/{slug}/merge-requests", web::get().to(merge_request::list_merge_requests))
            .route("/projects/{slug}/merge-requests", web::post().to(merge_request::create_merge_request))
            .route("/projects/{slug}/merge-requests/{iid}", web::get().to(merge_request::get_merge_request))
            .route("/projects/{slug}/merge-requests/{iid}", web::put().to(merge_request::update_merge_request))
            .route("/projects/{slug}/merge-requests/{iid}/merge", web::post().to(merge_request::merge))
            .route("/projects/{slug}/merge-requests/{iid}/close", web::post().to(merge_request::close))
            .route("/projects/{slug}/merge-requests/{iid}/comments", web::get().to(merge_request::list_comments))
            .route("/projects/{slug}/merge-requests/{iid}/comments", web::post().to(merge_request::add_comment))
            .route("/projects/{slug}/merge-requests/{iid}/reviews", web::post().to(merge_request::add_review))
            
            // Pipeline routes
            .route("/projects/{slug}/pipelines", web::get().to(pipeline::list_pipelines))
            .route("/projects/{slug}/pipelines", web::post().to(pipeline::trigger_pipeline))
            .route("/projects/{slug}/pipelines/{id}", web::get().to(pipeline::get_pipeline))
            .route("/projects/{slug}/pipelines/{id}/cancel", web::post().to(pipeline::cancel_pipeline))
            .route("/projects/{slug}/pipelines/{id}/retry", web::post().to(pipeline::retry_pipeline))
            .route("/projects/{slug}/pipelines/{id}/jobs", web::get().to(pipeline::list_jobs))
            .route("/projects/{slug}/pipelines/{id}/jobs/{job_id}/log", web::get().to(pipeline::get_job_log))
            
            // Webhook routes
            .route("/projects/{slug}/webhooks", web::get().to(webhook::list_webhooks))
            .route("/projects/{slug}/webhooks", web::post().to(webhook::create_webhook))
            .route("/projects/{slug}/webhooks/{id}", web::get().to(webhook::get_webhook))
            .route("/projects/{slug}/webhooks/{id}", web::put().to(webhook::update_webhook))
            .route("/projects/{slug}/webhooks/{id}", web::delete().to(webhook::delete_webhook))
            .route("/projects/{slug}/webhooks/{id}/test", web::post().to(webhook::test_webhook))
    );
}
