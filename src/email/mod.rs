/// Email sending functionality
///
/// This module provides email sending capabilities using lettre with SMTP.
/// Supports both real SMTP sending and mock mode for development/testing.

pub mod smtp;
pub mod templates;

use lettre::{Message, SmtpTransport, Transport};
use std::env;

/// Email sending result
pub type EmailResult<T> = Result<T, EmailError>;

/// Email errors
#[derive(Debug)]
pub enum EmailError {
    /// SMTP configuration error
    ConfigError(String),
    /// Email building error
    BuildError(lettre::error::Error),
    /// Email sending error
    SendError(lettre::transport::smtp::Error),
}

impl std::fmt::Display for EmailError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EmailError::ConfigError(msg) => write!(f, "Email config error: {}", msg),
            EmailError::BuildError(e) => write!(f, "Email build error: {}", e),
            EmailError::SendError(e) => write!(f, "Email send error: {}", e),
        }
    }
}

impl std::error::Error for EmailError {}

impl From<lettre::error::Error> for EmailError {
    fn from(e: lettre::error::Error) -> Self {
        EmailError::BuildError(e)
    }
}

impl From<lettre::transport::smtp::Error> for EmailError {
    fn from(e: lettre::transport::smtp::Error) -> Self {
        EmailError::SendError(e)
    }
}

/// Email configuration from environment variables
#[derive(Clone, Debug)]
pub struct EmailConfig {
    pub smtp_host: String,
    pub smtp_port: u16,
    pub smtp_username: String,
    pub smtp_password: String,
    pub from_email: String,
    pub from_name: String,
    pub use_tls: bool,
    pub mock: bool,
}

impl EmailConfig {
    /// Load email configuration from environment variables
    pub fn from_env() -> EmailResult<Self> {
        Ok(EmailConfig {
            smtp_host: env::var("SMTP_HOST")
                .unwrap_or_else(|_| "localhost".to_string()),
            smtp_port: env::var("SMTP_PORT")
                .unwrap_or_else(|_| "587".to_string())
                .parse()
                .map_err(|_| EmailError::ConfigError("Invalid SMTP_PORT".to_string()))?,
            smtp_username: env::var("SMTP_USERNAME")
                .unwrap_or_else(|_| "noreply@localhost".to_string()),
            smtp_password: env::var("SMTP_PASSWORD")
                .unwrap_or_else(|_| String::new()),
            from_email: env::var("SMTP_FROM_EMAIL")
                .unwrap_or_else(|_| "noreply@localhost".to_string()),
            from_name: env::var("SMTP_FROM_NAME")
                .unwrap_or_else(|_| "Ruforo Forum".to_string()),
            use_tls: env::var("SMTP_USE_TLS")
                .unwrap_or_else(|_| "true".to_string())
                .parse()
                .unwrap_or(true),
            mock: env::var("SMTP_MOCK")
                .unwrap_or_else(|_| "false".to_string())
                .parse()
                .unwrap_or(false),
        })
    }
}

/// Send an email
pub async fn send_email(
    to: &str,
    subject: &str,
    body_text: &str,
    body_html: Option<&str>,
) -> EmailResult<()> {
    let config = EmailConfig::from_env()?;

    if config.mock {
        // Mock mode: just log the email
        log::info!("MOCK EMAIL:");
        log::info!("  To: {}", to);
        log::info!("  Subject: {}", subject);
        log::info!("  Body: {}", body_text);
        return Ok(());
    }

    smtp::send_email(&config, to, subject, body_text, body_html).await
}
