pub mod admin;
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
pub mod issue;
pub mod personal_access_token;
pub mod oauth;
pub mod two_factor;
pub mod search;
pub mod runner;

use actix_web::{web, HttpResponse};
use serde::Serialize;
use crate::config::AppConfig;

/// Server configuration response (public info only)
#[derive(Serialize)]
pub struct ServerConfigResponse {
    pub ssh_enabled: bool,
    /// SSH clone URL prefix, e.g. "ssh://git@host:2222/" or "git@host:"
    pub ssh_clone_url_prefix: String,
    /// HTTP clone URL prefix, e.g. "http://localhost:8080/"
    pub http_clone_url_prefix: String,
}

/// GET /api/v1/config - Get public server configuration
pub async fn get_server_config(
    config: web::Data<AppConfig>,
    req: actix_web::HttpRequest,
) -> HttpResponse {
    let connection_info = req.connection_info();
    let http_clone_url_prefix = format!("{}://{}/", connection_info.scheme(), connection_info.host());
    
    // 根据端口决定 SSH URL 格式
    let ssh_clone_url_prefix = if config.ssh_enabled {
        let host = &config.ssh_public_host;
        let port = config.ssh_public_port;
        if port == 22 {
            // 默认端口使用 git@host: 格式
            format!("git@{}:", host)
        } else {
            // 非默认端口使用 ssh://git@host:port/ 格式
            format!("ssh://git@{}:{}/", host, port)
        }
    } else {
        String::new()
    };
    
    HttpResponse::Ok().json(ServerConfigResponse {
        ssh_enabled: config.ssh_enabled,
        ssh_clone_url_prefix,
        http_clone_url_prefix,
    })
}

pub fn configure_routes(cfg: &mut web::ServiceConfig) {
    // Git HTTP Smart Protocol routes (must be before API routes)
    git_http::configure_git_routes(cfg);
    
    // Internal API routes for GitFox Shell
    internal::configure_internal_routes(cfg);
    
    // Standard OAuth2 endpoints (at root level, not under /api/v1)
    cfg.route("/oauth/authorize", web::get().to(oauth::authorize))
       .route("/oauth/authorize", web::post().to(oauth::authorize_grant))
       .route("/oauth/token", web::post().to(oauth::token))
       // RFC 7009 Token Revocation
       .route("/oauth/revoke", web::post().to(oauth::revoke))
       // OIDC UserInfo Endpoint
       .route("/oauth/userinfo", web::get().to(oauth::userinfo));
    
    cfg.service(
        web::scope("/api/v1")
            // Server config route (public)
            .route("/config", web::get().to(get_server_config))
            
            // Global search route (public)
            .route("/search", web::get().to(search::search))
            
            // User's issues (requires authentication)
            .route("/issues", web::get().to(issue::my_issues))
            
            // Auth routes
            .route("/auth/register", web::post().to(auth::register))
            .route("/auth/login", web::post().to(auth::login))
            .route("/auth/verify-two-factor", web::post().to(auth::verify_two_factor))
            // Passkey direct login (no password)
            .route("/auth/passkey/login/start", web::post().to(auth::passkey_login_start))
            .route("/auth/passkey/login/finish", web::post().to(auth::passkey_login_finish))
            // WebAuthn for 2FA (after password login)
            .route("/auth/webauthn/start", web::post().to(auth::webauthn_auth_start))
            .route("/auth/webauthn/finish", web::post().to(auth::webauthn_auth_finish))
            .route("/auth/me", web::get().to(auth::me))
            .route("/auth/confirm-email", web::post().to(auth::confirm_email))
            .route("/auth/resend-confirmation", web::post().to(auth::resend_confirmation))
            .route("/auth/forgot-password", web::post().to(auth::forgot_password))
            .route("/auth/verify-reset-token", web::post().to(auth::verify_reset_token))
            .route("/auth/reset-password", web::post().to(auth::reset_password))
            
            // Admin routes (require admin role)
            .route("/admin/dashboard", web::get().to(admin::dashboard))
            .route("/admin/users", web::get().to(admin::list_users))
            .route("/admin/users/{id}", web::get().to(admin::get_user))
            .route("/admin/users/{id}", web::put().to(admin::update_user))
            .route("/admin/users/{id}", web::delete().to(admin::delete_user))
            .route("/admin/settings/configs", web::get().to(admin::get_configs))
            .route("/admin/settings/configs", web::put().to(admin::update_configs))
            // Admin SMTP settings
            .route("/admin/settings/smtp", web::get().to(admin::get_smtp_config))
            .route("/admin/settings/smtp/test", web::post().to(admin::test_smtp_connection))
            .route("/admin/settings/smtp/send-test", web::post().to(admin::send_test_email))
            // Admin OAuth provider management
            .route("/admin/oauth/providers", web::get().to(oauth::admin_list_providers))
            .route("/admin/oauth/providers", web::post().to(oauth::admin_create_provider))
            .route("/admin/oauth/providers/{id}", web::get().to(oauth::admin_get_provider))
            .route("/admin/oauth/providers/{id}", web::put().to(oauth::admin_update_provider))
            .route("/admin/oauth/providers/{id}", web::delete().to(oauth::admin_delete_provider))
            
            // Admin CI/CD Runner management (system-level runners)
            .route("/admin/runners", web::get().to(runner::admin_list_runners))
            .route("/admin/runners", web::post().to(runner::admin_create_runner))
            .route("/admin/runners/{id}", web::put().to(runner::admin_update_runner))
            .route("/admin/runners/{id}", web::delete().to(runner::admin_delete_runner))
            
            // User routes
            .route("/users", web::get().to(user::list_users))
            .route("/users/avatars", web::post().to(user::get_avatars_by_emails))
            .route("/users/{username}", web::get().to(user::get_user_by_username))
            .route("/users/{id}", web::put().to(user::update_user))
            .route("/users/{id}", web::delete().to(user::delete_user))
            
            // Current user profile and avatar routes
            .route("/user/profile", web::put().to(user::update_current_user_profile))
            .route("/user/avatar", web::post().to(user::upload_avatar))
            
            // Two-Factor Authentication routes
            .configure(two_factor::configure_routes)
            
            // SSH Key routes for current user
            .route("/user/ssh_keys", web::get().to(ssh_key::list_ssh_keys))
            .route("/user/ssh_keys", web::post().to(ssh_key::create_ssh_key))
            .route("/user/ssh_keys/{id}", web::get().to(ssh_key::get_ssh_key))
            .route("/user/ssh_keys/{id}", web::delete().to(ssh_key::delete_ssh_key))
            
            // Personal Access Token routes for current user
            .route("/user/access_tokens", web::get().to(personal_access_token::list_tokens))
            .route("/user/access_tokens", web::post().to(personal_access_token::create_token))
            .route("/user/access_tokens/scopes", web::get().to(personal_access_token::list_scopes))
            .route("/user/access_tokens/{id}", web::get().to(personal_access_token::get_token))
            .route("/user/access_tokens/{id}", web::delete().to(personal_access_token::revoke_token))
            
            // OAuth Identity routes for current user (linked social accounts)
            .route("/user/identities", web::get().to(oauth::list_identities))
            .route("/user/identities/{id}", web::delete().to(oauth::unlink_identity))
            
            // User CI/CD Runner management (user-level private runners)
            .route("/user/runners", web::get().to(runner::user_list_runners))
            .route("/user/runners", web::post().to(runner::user_create_runner))
            .route("/user/runners/{id}", web::put().to(runner::user_update_runner))
            .route("/user/runners/{id}", web::delete().to(runner::user_delete_runner))
            
            // OAuth Application routes (GitFox as OAuth provider)
            .route("/oauth/applications", web::get().to(oauth::list_applications))
            .route("/oauth/applications", web::post().to(oauth::create_application))
            .route("/oauth/applications/{id}", web::get().to(oauth::get_application))
            .route("/oauth/applications/{id}", web::put().to(oauth::update_application))
            .route("/oauth/applications/{id}", web::delete().to(oauth::delete_application))
            .route("/oauth/applications/{id}/regenerate_secret", web::post().to(oauth::regenerate_secret))
            
            // OAuth Authorization API (for frontend consent screen)
            .route("/oauth/authorize/info", web::get().to(oauth::authorize_info))
            .route("/oauth/authorize/confirm", web::post().to(oauth::authorize_confirm))
            
            // OAuth providers list (for social login buttons)
            .route("/oauth/providers", web::get().to(oauth::list_providers))
            
            // OAuth provider redirect endpoints (social login)
            .route("/oauth/{provider}/authorize", web::get().to(oauth::provider_authorize))
            .route("/oauth/{provider}/callback", web::get().to(oauth::provider_callback))
            
            // Project routes (style: /projects/:namespace/:project)
            .route("/projects", web::get().to(project::list_projects))
            .route("/projects", web::post().to(project::create_project))
            
            // Single project routes by namespace/project_name 
            .route("/projects/{namespace}/{project}", web::get().to(project::get_project))
            .route("/projects/{namespace}/{project}", web::put().to(project::update_project))
            .route("/projects/{namespace}/{project}", web::delete().to(project::delete_project))
            .route("/projects/{namespace}/{project}/stats", web::get().to(project::get_project_stats))
            .route("/projects/{namespace}/{project}/members", web::get().to(project::get_members))
            .route("/projects/{namespace}/{project}/members", web::post().to(project::add_member))
            .route("/projects/{namespace}/{project}/members/{user_id}", web::delete().to(project::remove_member))
            
            // Star routes
            .route("/projects/{namespace}/{project}/starred", web::get().to(project::check_starred))
            .route("/projects/{namespace}/{project}/star", web::post().to(project::star_project))
            .route("/projects/{namespace}/{project}/star", web::delete().to(project::unstar_project))
            
            // Fork routes
            .route("/projects/{namespace}/{project}/fork", web::post().to(project::fork_project))
            .route("/projects/{namespace}/{project}/forks", web::get().to(project::list_forks))
            
            // Repository routes 
            .route("/projects/{namespace}/{project}/repository", web::get().to(repository::get_repository_info))
            .route("/projects/{namespace}/{project}/repository/tree", web::get().to(repository::browse_tree))
            .route("/projects/{namespace}/{project}/repository/files/{filepath:.*}", web::get().to(repository::get_file))
            .route("/projects/{namespace}/{project}/repository/blobs/{sha}", web::get().to(repository::get_blob))
            
            // Branch routes 
            .route("/projects/{namespace}/{project}/repository/branches", web::get().to(branch::list_branches))
            .route("/projects/{namespace}/{project}/repository/branches", web::post().to(branch::create_branch))
            .route("/projects/{namespace}/{project}/repository/branches/{branch:.*}", web::get().to(branch::get_branch))
            .route("/projects/{namespace}/{project}/repository/branches/{branch:.*}", web::delete().to(branch::delete_branch))
            
            // Commit routes 
            .route("/projects/{namespace}/{project}/repository/commits", web::get().to(commit::list_commits))
            .route("/projects/{namespace}/{project}/repository/commits/{sha}/files/{file_path:.*}", web::get().to(commit::get_full_file_diff))
            .route("/projects/{namespace}/{project}/repository/commits/{sha}", web::get().to(commit::get_commit))
            .route("/projects/{namespace}/{project}/repository/compare", web::get().to(commit::compare))
            
            // Tag routes 
            .route("/projects/{namespace}/{project}/repository/tags", web::get().to(tag::list_tags))
            .route("/projects/{namespace}/{project}/repository/tags", web::post().to(tag::create_tag))
            .route("/projects/{namespace}/{project}/repository/tags/{tag_name}", web::get().to(tag::get_tag))
            .route("/projects/{namespace}/{project}/repository/tags/{tag_name}", web::delete().to(tag::delete_tag))
            
            // Merge Request routes 
            .route("/projects/{namespace}/{project}/merge_requests", web::get().to(merge_request::list_merge_requests))
            .route("/projects/{namespace}/{project}/merge_requests", web::post().to(merge_request::create_merge_request))
            .route("/projects/{namespace}/{project}/merge_requests/{iid}", web::get().to(merge_request::get_merge_request))
            .route("/projects/{namespace}/{project}/merge_requests/{iid}", web::put().to(merge_request::update_merge_request))
            .route("/projects/{namespace}/{project}/merge_requests/{iid}/merge", web::put().to(merge_request::merge))
            .route("/projects/{namespace}/{project}/merge_requests/{iid}/notes", web::get().to(merge_request::list_comments))
            .route("/projects/{namespace}/{project}/merge_requests/{iid}/notes", web::post().to(merge_request::add_comment))
            
            // Issue routes
            .route("/projects/{namespace}/{project}/issues", web::get().to(issue::list_issues))
            .route("/projects/{namespace}/{project}/issues", web::post().to(issue::create_issue))
            .route("/projects/{namespace}/{project}/issues/{iid}", web::get().to(issue::get_issue))
            .route("/projects/{namespace}/{project}/issues/{iid}", web::put().to(issue::update_issue))
            .route("/projects/{namespace}/{project}/issues/{iid}", web::delete().to(issue::delete_issue))
            .route("/projects/{namespace}/{project}/issues/{iid}/notes", web::get().to(issue::list_issue_notes))
            .route("/projects/{namespace}/{project}/issues/{iid}/notes", web::post().to(issue::add_issue_note))
            
            // Pipeline routes 
            .route("/projects/{namespace}/{project}/pipelines", web::get().to(pipeline::list_pipelines))
            .route("/projects/{namespace}/{project}/pipelines", web::post().to(pipeline::trigger_pipeline))
            .route("/projects/{namespace}/{project}/pipelines/{id}", web::get().to(pipeline::get_pipeline))
            .route("/projects/{namespace}/{project}/pipelines/{id}/cancel", web::post().to(pipeline::cancel_pipeline))
            .route("/projects/{namespace}/{project}/pipelines/{id}/retry", web::post().to(pipeline::retry_pipeline))
            .route("/projects/{namespace}/{project}/jobs", web::get().to(pipeline::list_jobs))
            .route("/projects/{namespace}/{project}/jobs/{job_id}/trace", web::get().to(pipeline::get_job_log))
            
            // Webhook/Hooks routes 
            .route("/projects/{namespace}/{project}/hooks", web::get().to(webhook::list_webhooks))
            .route("/projects/{namespace}/{project}/hooks", web::post().to(webhook::create_webhook))
            .route("/projects/{namespace}/{project}/hooks/{id}", web::get().to(webhook::get_webhook))
            .route("/projects/{namespace}/{project}/hooks/{id}", web::put().to(webhook::update_webhook))
            .route("/projects/{namespace}/{project}/hooks/{id}", web::delete().to(webhook::delete_webhook))
            .route("/projects/{namespace}/{project}/hooks/{id}/test", web::post().to(webhook::test_webhook))
            
            // Project CI/CD Runner management
            .route("/projects/{namespace}/{project}/runners", web::get().to(runner::project_list_runners))
            .route("/projects/{namespace}/{project}/runners", web::post().to(runner::project_create_runner))
            .route("/projects/{namespace}/{project}/runners/{id}", web::put().to(runner::project_update_runner))
            .route("/projects/{namespace}/{project}/runners/{id}", web::delete().to(runner::project_delete_runner))
            
            // Namespace/Group routes - specific routes MUST come before generic {path:.*} routes
            .route("/groups", web::get().to(namespace::list_groups))
            .route("/groups", web::post().to(namespace::create_group))
            // Routes with suffixes must come before the generic get/put/delete
            .route("/groups/{path:.*}/members/{user_id}", web::delete().to(namespace::remove_group_member))
            .route("/groups/{path:.*}/members", web::get().to(namespace::list_group_members))
            .route("/groups/{path:.*}/members", web::post().to(namespace::add_group_member))
            .route("/groups/{path:.*}/projects", web::get().to(namespace::list_group_projects))
            .route("/groups/{path:.*}/subgroups", web::get().to(namespace::list_subgroups))
            // Namespace CI/CD Runner management
            .route("/groups/{path:.*}/runners", web::get().to(runner::namespace_list_runners))
            .route("/groups/{path:.*}/runners", web::post().to(runner::namespace_create_runner))
            .route("/groups/{path:.*}/runners/{id}", web::put().to(runner::namespace_update_runner))
            .route("/groups/{path:.*}/runners/{id}", web::delete().to(runner::namespace_delete_runner))
            // Generic group routes come last
            .route("/groups/{path:.*}", web::get().to(namespace::get_group))
            .route("/groups/{path:.*}", web::put().to(namespace::update_group))
            .route("/groups/{path:.*}", web::delete().to(namespace::delete_group))
            
            // Namespaces (users + groups unified listing)
            .route("/namespaces", web::get().to(namespace::list_namespaces))
            .route("/namespaces/for-project-creation", web::get().to(namespace::list_namespaces_for_project_creation))
            .route("/namespaces/{path:.*}", web::get().to(namespace::get_namespace))
            
            // CI/CD Runner endpoints
            .route("/runner/register", web::post().to(runner::runner_register))
            .route("/runner/connect", web::get().to(runner::runner_connect))
    );
}
