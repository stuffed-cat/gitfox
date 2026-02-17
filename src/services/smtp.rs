use lettre::{
    message::{header::ContentType, Mailbox},
    transport::smtp::authentication::Credentials,
    AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
};
use serde::{Deserialize, Serialize};

use crate::config::SmtpConfig;
use crate::error::{AppError, AppResult};

/// SMTP settings from environment config
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SmtpSettings {
    pub enabled: bool,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub from_email: String,
    pub from_name: String,
    pub use_tls: bool,
    pub use_ssl: bool,
}

impl SmtpSettings {
    /// Load SMTP settings from environment config
    pub fn from_config(env_config: &SmtpConfig) -> Self {
        Self {
            enabled: env_config.enabled,
            host: env_config.host.clone().unwrap_or_default(),
            port: env_config.port,
            username: env_config.username.clone().unwrap_or_default(),
            password: env_config.password.clone().unwrap_or_default(),
            from_email: env_config.from_email.clone(),
            from_name: env_config.from_name.clone(),
            use_tls: env_config.use_tls,
            use_ssl: env_config.use_ssl,
        }
    }

    /// Check if SMTP is properly configured
    pub fn is_configured(&self) -> bool {
        self.enabled && !self.host.is_empty()
    }
}

/// SMTP service for sending emails
pub struct SmtpService;

impl SmtpService {
    /// Create an SMTP transport from settings
    fn create_transport(settings: &SmtpSettings) -> AppResult<AsyncSmtpTransport<Tokio1Executor>> {
        if settings.host.is_empty() {
            return Err(AppError::BadRequest("SMTP host is not configured".to_string()));
        }

        let mut builder = if settings.use_ssl {
            // Implicit TLS (port 465)
            AsyncSmtpTransport::<Tokio1Executor>::relay(&settings.host)?
                .port(settings.port)
        } else if settings.use_tls {
            // STARTTLS (port 587)
            AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&settings.host)?
                .port(settings.port)
        } else {
            // No encryption (port 25)
            AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(&settings.host)
                .port(settings.port)
        };

        // Add credentials if provided
        if !settings.username.is_empty() {
            let credentials = Credentials::new(
                settings.username.clone(),
                settings.password.clone(),
            );
            builder = builder.credentials(credentials);
        }

        Ok(builder.build())
    }

    /// Send an email
    pub async fn send_email(
        env_config: &SmtpConfig,
        to_email: &str,
        to_name: Option<&str>,
        subject: &str,
        body_html: &str,
        _body_text: Option<&str>,
    ) -> AppResult<()> {
        let settings = SmtpSettings::from_config(env_config);
        
        if !settings.is_configured() {
            log::warn!("SMTP is not configured, email not sent to {}", to_email);
            return Err(AppError::BadRequest("SMTP is not configured".to_string()));
        }

        let from_mailbox: Mailbox = format!("{} <{}>", settings.from_name, settings.from_email)
            .parse()
            .map_err(|e| AppError::InternalError(format!("Invalid from email: {}", e)))?;

        let to_display = to_name.unwrap_or(to_email);
        let to_mailbox: Mailbox = format!("{} <{}>", to_display, to_email)
            .parse()
            .map_err(|e| AppError::InternalError(format!("Invalid to email: {}", e)))?;

        let email = Message::builder()
            .from(from_mailbox)
            .to(to_mailbox)
            .subject(subject)
            .header(ContentType::TEXT_HTML)
            .body(body_html.to_string())
            .map_err(|e| AppError::InternalError(format!("Failed to build email: {}", e)))?;

        let transport = Self::create_transport(&settings)?;
        transport.send(email).await
            .map_err(|e| AppError::InternalError(format!("Failed to send email: {}", e)))?;

        log::info!("Email sent to {}: {}", to_email, subject);
        Ok(())
    }

    /// Test SMTP connection with given settings
    pub async fn test_connection(settings: &SmtpSettings) -> AppResult<()> {
        if settings.host.is_empty() {
            return Err(AppError::BadRequest("SMTP host is required".to_string()));
        }

        let transport = Self::create_transport(settings)?;
        transport.test_connection().await
            .map_err(|e| AppError::BadRequest(format!("SMTP connection failed: {}", e)))?;

        Ok(())
    }

    /// Send a test email
    pub async fn send_test_email(
        settings: &SmtpSettings,
        to_email: &str,
    ) -> AppResult<()> {
        if !settings.is_configured() {
            return Err(AppError::BadRequest("SMTP is not configured".to_string()));
        }

        let from_mailbox: Mailbox = format!("{} <{}>", settings.from_name, settings.from_email)
            .parse()
            .map_err(|e| AppError::InternalError(format!("Invalid from email: {}", e)))?;

        let to_mailbox: Mailbox = to_email.parse()
            .map_err(|e| AppError::InternalError(format!("Invalid to email: {}", e)))?;

        let body = r#"
<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <title>GitFox SMTP Test</title>
</head>
<body style="font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; padding: 20px;">
    <h1 style="color: #1f2937;">GitFox SMTP Test</h1>
    <p>This is a test email from GitFox to verify your SMTP configuration.</p>
    <p style="color: #22c55e; font-weight: bold;">✓ Your SMTP settings are working correctly!</p>
    <hr style="border: none; border-top: 1px solid #e5e7eb; margin: 20px 0;">
    <p style="color: #6b7280; font-size: 12px;">
        This email was sent automatically by GitFox. Please do not reply to this email.
    </p>
</body>
</html>
"#;

        let email = Message::builder()
            .from(from_mailbox)
            .to(to_mailbox)
            .subject("GitFox SMTP Test Email")
            .header(ContentType::TEXT_HTML)
            .body(body.to_string())
            .map_err(|e| AppError::InternalError(format!("Failed to build email: {}", e)))?;

        let transport = Self::create_transport(settings)?;
        transport.send(email).await
            .map_err(|e| AppError::BadRequest(format!("Failed to send test email: {}", e)))?;

        Ok(())
    }

    // ─── Email Templates ───────────────────────────────────

    /// Send email confirmation email
    pub async fn send_email_confirmation(
        env_config: &SmtpConfig,
        to_email: &str,
        username: &str,
        confirmation_token: &str,
        base_url: &str,
    ) -> AppResult<()> {
        let confirmation_url = format!("{}/confirm-email?token={}", base_url, confirmation_token);
        
        let body = format!(r#"
<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <title>确认您的邮箱</title>
</head>
<body style="font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; padding: 20px; background-color: #f9fafb;">
    <div style="max-width: 600px; margin: 0 auto; background: white; border-radius: 8px; padding: 40px; box-shadow: 0 1px 3px rgba(0,0,0,0.1);">
        <h1 style="color: #1f2937; margin-bottom: 20px;">欢迎加入 GitFox！</h1>
        <p style="color: #4b5563; line-height: 1.6;">
            您好 <strong>{username}</strong>，
        </p>
        <p style="color: #4b5563; line-height: 1.6;">
            感谢您注册 GitFox。请点击下面的按钮确认您的邮箱地址：
        </p>
        <div style="text-align: center; margin: 30px 0;">
            <a href="{confirmation_url}" 
               style="display: inline-block; padding: 12px 24px; background-color: #2563eb; color: white; text-decoration: none; border-radius: 6px; font-weight: 500;">
                确认邮箱
            </a>
        </div>
        <p style="color: #6b7280; font-size: 14px;">
            如果按钮无法点击，请复制以下链接到浏览器：
        </p>
        <p style="color: #2563eb; font-size: 14px; word-break: break-all;">
            {confirmation_url}
        </p>
        <hr style="border: none; border-top: 1px solid #e5e7eb; margin: 30px 0;">
        <p style="color: #9ca3af; font-size: 12px;">
            如果您没有注册 GitFox 账户，请忽略此邮件。
        </p>
    </div>
</body>
</html>
"#, username = username, confirmation_url = confirmation_url);

        Self::send_email(
            env_config,
            to_email,
            Some(username),
            "确认您的 GitFox 邮箱",
            &body,
            None,
        ).await
    }

    /// Send password reset email
    pub async fn send_password_reset(
        env_config: &SmtpConfig,
        to_email: &str,
        username: &str,
        reset_token: &str,
        base_url: &str,
    ) -> AppResult<()> {
        let reset_url = format!("{}/reset-password?token={}", base_url, reset_token);
        
        let body = format!(r#"
<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <title>重置您的密码</title>
</head>
<body style="font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; padding: 20px; background-color: #f9fafb;">
    <div style="max-width: 600px; margin: 0 auto; background: white; border-radius: 8px; padding: 40px; box-shadow: 0 1px 3px rgba(0,0,0,0.1);">
        <h1 style="color: #1f2937; margin-bottom: 20px;">重置您的密码</h1>
        <p style="color: #4b5563; line-height: 1.6;">
            您好 <strong>{username}</strong>，
        </p>
        <p style="color: #4b5563; line-height: 1.6;">
            我们收到了重置您 GitFox 账户密码的请求。请点击下面的按钮设置新密码：
        </p>
        <div style="text-align: center; margin: 30px 0;">
            <a href="{reset_url}" 
               style="display: inline-block; padding: 12px 24px; background-color: #dc2626; color: white; text-decoration: none; border-radius: 6px; font-weight: 500;">
                重置密码
            </a>
        </div>
        <p style="color: #6b7280; font-size: 14px;">
            如果按钮无法点击，请复制以下链接到浏览器：
        </p>
        <p style="color: #dc2626; font-size: 14px; word-break: break-all;">
            {reset_url}
        </p>
        <p style="color: #f59e0b; font-size: 14px; margin-top: 20px;">
            ⚠️ 此链接将在 24 小时后失效。
        </p>
        <hr style="border: none; border-top: 1px solid #e5e7eb; margin: 30px 0;">
        <p style="color: #9ca3af; font-size: 12px;">
            如果您没有请求重置密码，请忽略此邮件，您的密码将保持不变。
        </p>
    </div>
</body>
</html>
"#, username = username, reset_url = reset_url);

        Self::send_email(
            env_config,
            to_email,
            Some(username),
            "重置您的 GitFox 密码",
            &body,
            None,
        ).await
    }

    /// Send new issue notification
    pub async fn send_issue_notification(
        env_config: &SmtpConfig,
        to_email: &str,
        to_name: Option<&str>,
        project_name: &str,
        issue_title: &str,
        issue_number: i64,
        author_name: &str,
        issue_url: &str,
    ) -> AppResult<()> {
        let body = format!(r#"
<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <title>新 Issue 通知</title>
</head>
<body style="font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; padding: 20px; background-color: #f9fafb;">
    <div style="max-width: 600px; margin: 0 auto; background: white; border-radius: 8px; padding: 40px; box-shadow: 0 1px 3px rgba(0,0,0,0.1);">
        <p style="color: #6b7280; font-size: 14px; margin-bottom: 20px;">
            {project_name}
        </p>
        <h1 style="color: #1f2937; margin-bottom: 10px; font-size: 20px;">
            #{issue_number} {issue_title}
        </h1>
        <p style="color: #4b5563; line-height: 1.6;">
            <strong>{author_name}</strong> 创建了一个新的 Issue
        </p>
        <div style="text-align: center; margin: 30px 0;">
            <a href="{issue_url}" 
               style="display: inline-block; padding: 12px 24px; background-color: #2563eb; color: white; text-decoration: none; border-radius: 6px; font-weight: 500;">
                查看 Issue
            </a>
        </div>
        <hr style="border: none; border-top: 1px solid #e5e7eb; margin: 30px 0;">
        <p style="color: #9ca3af; font-size: 12px;">
            您收到此邮件是因为您关注了此项目或被提及。
        </p>
    </div>
</body>
</html>
"#, 
            project_name = project_name,
            issue_number = issue_number,
            issue_title = issue_title,
            author_name = author_name,
            issue_url = issue_url
        );

        Self::send_email(
            env_config,
            to_email,
            to_name,
            &format!("[{}] 新 Issue #{}: {}", project_name, issue_number, issue_title),
            &body,
            None,
        ).await
    }
}
