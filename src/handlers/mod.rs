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
pub mod git_http;
pub mod namespace;
pub mod ssh_key;
pub mod internal;

use actix_web::web;

pub fn configure_routes(cfg: &mut web::ServiceConfig) {
    // Git HTTP Smart Protocol routes (must be before API routes)
    git_http::configure_git_routes(cfg);
    
    // Internal API routes for GitFox Shell
    internal::configure_internal_routes(cfg);
    
    cfg.service(
        web::scope("/api/v1")
            // Auth routes
            .route("/auth/register", web::post().to(auth::register))
            .route("/auth/login", web::post().to(auth::login))
            .route("/auth/me", web::get().to(auth::me))
            
            // User routes
            .route("/users", web::get().to(user::list_users))
            .route("/users/{username}", web::get().to(user::get_user_by_username))
            .route("/users/{id}", web::put().to(user::update_user))
            .route("/users/{id}", web::delete().to(user::delete_user))
            
            // SSH Key routes for current user
            .route("/user/ssh_keys", web::get().to(ssh_key::list_ssh_keys))
            .route("/user/ssh_keys", web::post().to(ssh_key::create_ssh_key))
            .route("/user/ssh_keys/{id}", web::get().to(ssh_key::get_ssh_key))
            .route("/user/ssh_keys/{id}", web::delete().to(ssh_key::delete_ssh_key))
            
            // Project routes (style: /projects/:namespace/:project)
            .route("/projects", web::get().to(project::list_projects))
            .route("/projects", web::post().to(project::create_project))
            
            // Single project routes by namespace/project_name (GitLab v4 style)
            .route("/projects/{namespace}/{project}", web::get().to(project::get_project))
            .route("/projects/{namespace}/{project}", web::put().to(project::update_project))
            .route("/projects/{namespace}/{project}", web::delete().to(project::delete_project))
            .route("/projects/{namespace}/{project}/stats", web::get().to(project::get_project_stats))
            .route("/projects/{namespace}/{project}/members", web::get().to(project::get_members))
            .route("/projects/{namespace}/{project}/members", web::post().to(project::add_member))
            .route("/projects/{namespace}/{project}/members/{user_id}", web::delete().to(project::remove_member))
            
            // Repository routes (GitLab v4 style)
            .route("/projects/{namespace}/{project}/repository", web::get().to(repository::get_repository_info))
            .route("/projects/{namespace}/{project}/repository/tree", web::get().to(repository::browse_tree))
            .route("/projects/{namespace}/{project}/repository/files/{filepath:.*}", web::get().to(repository::get_file))
            .route("/projects/{namespace}/{project}/repository/blobs/{sha}", web::get().to(repository::get_blob))
            
            // Branch routes (GitLab v4 style)
            .route("/projects/{namespace}/{project}/repository/branches", web::get().to(branch::list_branches))
            .route("/projects/{namespace}/{project}/repository/branches", web::post().to(branch::create_branch))
            .route("/projects/{namespace}/{project}/repository/branches/{branch:.*}", web::get().to(branch::get_branch))
            .route("/projects/{namespace}/{project}/repository/branches/{branch:.*}", web::delete().to(branch::delete_branch))
            
            // Commit routes (GitLab v4 style)
            .route("/projects/{namespace}/{project}/repository/commits", web::get().to(commit::list_commits))
            .route("/projects/{namespace}/{project}/repository/commits/{sha}", web::get().to(commit::get_commit))
            .route("/projects/{namespace}/{project}/repository/compare", web::get().to(commit::compare))
            
            // Tag routes (GitLab v4 style)
            .route("/projects/{namespace}/{project}/repository/tags", web::get().to(tag::list_tags))
            .route("/projects/{namespace}/{project}/repository/tags", web::post().to(tag::create_tag))
            .route("/projects/{namespace}/{project}/repository/tags/{tag_name}", web::get().to(tag::get_tag))
            .route("/projects/{namespace}/{project}/repository/tags/{tag_name}", web::delete().to(tag::delete_tag))
            
            // Merge Request routes (GitLab v4 style)
            .route("/projects/{namespace}/{project}/merge_requests", web::get().to(merge_request::list_merge_requests))
            .route("/projects/{namespace}/{project}/merge_requests", web::post().to(merge_request::create_merge_request))
            .route("/projects/{namespace}/{project}/merge_requests/{iid}", web::get().to(merge_request::get_merge_request))
            .route("/projects/{namespace}/{project}/merge_requests/{iid}", web::put().to(merge_request::update_merge_request))
            .route("/projects/{namespace}/{project}/merge_requests/{iid}/merge", web::put().to(merge_request::merge))
            .route("/projects/{namespace}/{project}/merge_requests/{iid}/notes", web::get().to(merge_request::list_comments))
            .route("/projects/{namespace}/{project}/merge_requests/{iid}/notes", web::post().to(merge_request::add_comment))
            
            // Pipeline routes (GitLab v4 style)
            .route("/projects/{namespace}/{project}/pipelines", web::get().to(pipeline::list_pipelines))
            .route("/projects/{namespace}/{project}/pipelines", web::post().to(pipeline::trigger_pipeline))
            .route("/projects/{namespace}/{project}/pipelines/{id}", web::get().to(pipeline::get_pipeline))
            .route("/projects/{namespace}/{project}/pipelines/{id}/cancel", web::post().to(pipeline::cancel_pipeline))
            .route("/projects/{namespace}/{project}/pipelines/{id}/retry", web::post().to(pipeline::retry_pipeline))
            .route("/projects/{namespace}/{project}/jobs", web::get().to(pipeline::list_jobs))
            .route("/projects/{namespace}/{project}/jobs/{job_id}/trace", web::get().to(pipeline::get_job_log))
            
            // Webhook/Hooks routes (GitLab v4 style)
            .route("/projects/{namespace}/{project}/hooks", web::get().to(webhook::list_webhooks))
            .route("/projects/{namespace}/{project}/hooks", web::post().to(webhook::create_webhook))
            .route("/projects/{namespace}/{project}/hooks/{id}", web::get().to(webhook::get_webhook))
            .route("/projects/{namespace}/{project}/hooks/{id}", web::put().to(webhook::update_webhook))
            .route("/projects/{namespace}/{project}/hooks/{id}", web::delete().to(webhook::delete_webhook))
            .route("/projects/{namespace}/{project}/hooks/{id}/test", web::post().to(webhook::test_webhook))
            
            // Namespace/Group routes
            .route("/groups", web::get().to(namespace::list_groups))
            .route("/groups", web::post().to(namespace::create_group))
            .route("/groups/{path:.*}", web::get().to(namespace::get_group))
            .route("/groups/{path:.*}", web::put().to(namespace::update_group))
            .route("/groups/{path:.*}", web::delete().to(namespace::delete_group))
            .route("/groups/{path:.*}/members", web::get().to(namespace::list_group_members))
            .route("/groups/{path:.*}/members", web::post().to(namespace::add_group_member))
            .route("/groups/{path:.*}/members/{user_id}", web::delete().to(namespace::remove_group_member))
            .route("/groups/{path:.*}/projects", web::get().to(namespace::list_group_projects))
            .route("/groups/{path:.*}/subgroups", web::get().to(namespace::list_subgroups))
            
            // Namespaces (users + groups unified listing)
            .route("/namespaces", web::get().to(namespace::list_namespaces))
            .route("/namespaces/{path:.*}", web::get().to(namespace::get_namespace))
    );
}
