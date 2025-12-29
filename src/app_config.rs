//! Application configuration from file and environment variables
//!
//! Configuration is loaded with the following priority (highest to lowest):
//! 1. Environment variables (prefixed with RUFORO_)
//! 2. Config file (config.toml)
//! 3. Default values
//!
//! Secrets like database passwords and API keys should be kept in environment
//! variables, not in the config file.

use config::{Config, ConfigError, Environment, File};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::sync::RwLock;

/// Global application configuration
pub static APP_CONFIG: Lazy<RwLock<AppConfig>> = Lazy::new(|| {
    RwLock::new(AppConfig::load().unwrap_or_else(|e| {
        log::warn!("Failed to load config file, using defaults: {}", e);
        AppConfig::default()
    }))
});

/// Site configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SiteConfig {
    pub name: String,
    pub description: String,
    pub base_url: String,
}

impl Default for SiteConfig {
    fn default() -> Self {
        Self {
            name: "Ruforo".to_string(),
            description: "A forum built in Rust".to_string(),
            base_url: "http://localhost:8080".to_string(),
        }
    }
}

/// CAPTCHA configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct CaptchaConfig {
    /// Provider: "hcaptcha", "turnstile", or empty to disable
    pub provider: String,
    /// Public site key (can be in config file)
    pub site_key: String,
    /// Secret key (should be in env var RUFORO_CAPTCHA_SECRET_KEY)
    #[serde(default)]
    pub secret_key: String,
    /// Number of failed login attempts before requiring CAPTCHA
    pub failed_login_threshold: u32,
}

impl Default for CaptchaConfig {
    fn default() -> Self {
        Self {
            provider: String::new(),
            site_key: String::new(),
            secret_key: String::new(),
            failed_login_threshold: 3,
        }
    }
}

/// Security configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SecurityConfig {
    /// Maximum failed login attempts before account lockout
    pub max_failed_logins: u32,
    /// Account lockout duration in minutes
    pub lockout_duration_minutes: u32,
    /// Session timeout in minutes (default: 24 hours)
    pub session_timeout_minutes: u32,
    /// Remember me session duration in days
    pub remember_me_days: u32,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            max_failed_logins: 5,
            lockout_duration_minutes: 15,
            session_timeout_minutes: 1440,
            remember_me_days: 30,
        }
    }
}

/// Rate limiting configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct RateLimitConfig {
    /// Login attempts per window
    pub login_max_attempts: u32,
    /// Login rate limit window in seconds
    pub login_window_seconds: u32,
    /// Registration attempts per hour
    pub registration_per_hour: u32,
    /// Posts per minute per user
    pub posts_per_minute: u32,
    /// Threads per 5 minutes per user
    pub threads_per_5_minutes: u32,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            login_max_attempts: 5,
            login_window_seconds: 300,
            registration_per_hour: 3,
            posts_per_minute: 10,
            threads_per_5_minutes: 5,
        }
    }
}

/// Content limits configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct LimitsConfig {
    /// Posts per page (default)
    pub posts_per_page: u32,
    /// Threads per page (default)
    pub threads_per_page: u32,
    /// Maximum upload size in MB
    pub max_upload_size_mb: u32,
    /// Maximum post length for regular users
    pub max_post_length: u32,
    /// Maximum post length for moderators
    pub max_post_length_mod: u32,
}

impl Default for LimitsConfig {
    fn default() -> Self {
        Self {
            posts_per_page: 25,
            threads_per_page: 20,
            max_upload_size_mb: 10,
            max_post_length: 50000,
            max_post_length_mod: 100000,
        }
    }
}

/// Email configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct EmailConfig {
    /// SMTP server host
    pub smtp_host: String,
    /// SMTP server port
    pub smtp_port: u16,
    /// Use TLS for SMTP
    pub smtp_tls: bool,
    /// SMTP username (if required)
    pub smtp_username: String,
    /// SMTP password (should be in env var RUFORO_EMAIL_SMTP_PASSWORD)
    #[serde(default)]
    pub smtp_password: String,
    /// From address for emails
    pub from_address: String,
    /// From name for emails
    pub from_name: String,
}

impl Default for EmailConfig {
    fn default() -> Self {
        Self {
            smtp_host: "localhost".to_string(),
            smtp_port: 587,
            smtp_tls: true,
            smtp_username: String::new(),
            smtp_password: String::new(),
            from_address: "noreply@localhost".to_string(),
            from_name: "Ruforo".to_string(),
        }
    }
}

/// Storage configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct StorageConfig {
    /// Storage backend: "local" or "s3"
    pub backend: String,
    /// Local storage path (used when backend = "local")
    pub local_path: String,
    /// S3 endpoint URL (used when backend = "s3")
    pub s3_endpoint: String,
    /// S3 region (used when backend = "s3")
    pub s3_region: String,
    /// S3 bucket name (used when backend = "s3")
    pub s3_bucket: String,
    /// S3 public URL for serving files (used when backend = "s3")
    pub s3_public_url: String,
    /// S3 access key (should be in env var RUFORO_STORAGE_S3_ACCESS_KEY)
    #[serde(default)]
    pub s3_access_key: String,
    /// S3 secret key (should be in env var RUFORO_STORAGE_S3_SECRET_KEY)
    #[serde(default)]
    pub s3_secret_key: String,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            backend: "local".to_string(),
            local_path: "./uploads".to_string(),
            s3_endpoint: "http://localhost:9000".to_string(),
            s3_region: "us-east-1".to_string(),
            s3_bucket: "ruforo".to_string(),
            s3_public_url: "http://localhost:9000/ruforo".to_string(),
            s3_access_key: String::new(),
            s3_secret_key: String::new(),
        }
    }
}

/// Spam detection configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SpamConfig {
    /// Spam score threshold (0.0 - 1.0)
    pub threshold: f32,
    /// Enable spam detection
    pub enabled: bool,
    /// Maximum URLs allowed before flagging
    pub max_urls: u32,
    /// Block first posts with URLs
    pub block_first_post_urls: bool,
}

impl Default for SpamConfig {
    fn default() -> Self {
        Self {
            threshold: 0.7,
            enabled: true,
            max_urls: 5,
            block_first_post_urls: false,
        }
    }
}

/// Main application configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct AppConfig {
    pub site: SiteConfig,
    pub captcha: CaptchaConfig,
    pub security: SecurityConfig,
    pub rate_limit: RateLimitConfig,
    pub limits: LimitsConfig,
    pub email: EmailConfig,
    pub storage: StorageConfig,
    pub spam: SpamConfig,
}

impl AppConfig {
    /// Load configuration from file and environment variables
    pub fn load() -> Result<Self, ConfigError> {
        Self::load_from_path("config.toml")
    }

    /// Load configuration from a specific path
    pub fn load_from_path(path: &str) -> Result<Self, ConfigError> {
        use config::FileFormat;

        let config = Config::builder()
            // Start with defaults
            .add_source(config::Config::try_from(&AppConfig::default())?)
            // Add config file (optional) - use from_file for full path support
            .add_source(File::new(path, FileFormat::Toml).required(false))
            // Override with environment variables (RUFORO_ prefix)
            // e.g., RUFORO_CAPTCHA_PROVIDER, RUFORO_SITE_NAME
            .add_source(
                Environment::with_prefix("RUFORO")
                    .separator("_")
                    .try_parsing(true),
            )
            .build()?;

        config.try_deserialize()
    }

    /// Reload configuration from file
    pub fn reload() -> Result<(), ConfigError> {
        let new_config = Self::load()?;
        if let Ok(mut config) = APP_CONFIG.write() {
            *config = new_config;
            log::info!("Configuration reloaded");
        }
        Ok(())
    }
}

/// Initialize application configuration
///
/// This triggers the lazy loading of the config file and logs the result.
/// Should be called early in application startup.
pub fn init() {
    // Access the lazy static to trigger initialization
    let config = APP_CONFIG.read().unwrap();
    log::info!("Configuration loaded: site.name = {}", config.site.name);
}

// Convenience functions for accessing global config

/// Get the current application configuration
pub fn get_config() -> AppConfig {
    APP_CONFIG.read().map(|c| c.clone()).unwrap_or_default()
}

/// Get site configuration
pub fn site() -> SiteConfig {
    get_config().site
}

/// Get CAPTCHA configuration
pub fn captcha() -> CaptchaConfig {
    get_config().captcha
}

/// Get security configuration
pub fn security() -> SecurityConfig {
    get_config().security
}

/// Get rate limit configuration
pub fn rate_limit() -> RateLimitConfig {
    get_config().rate_limit
}

/// Get limits configuration
pub fn limits() -> LimitsConfig {
    get_config().limits
}

/// Get email configuration
pub fn email() -> EmailConfig {
    get_config().email
}

/// Get storage configuration
pub fn storage() -> StorageConfig {
    get_config().storage
}

/// Get spam configuration
pub fn spam() -> SpamConfig {
    get_config().spam
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_default_config() {
        let config = AppConfig::default();
        assert_eq!(config.site.name, "Ruforo");
        assert_eq!(config.captcha.failed_login_threshold, 3);
        assert_eq!(config.security.max_failed_logins, 5);
        assert_eq!(config.limits.posts_per_page, 25);
    }

    #[test]
    fn test_captcha_disabled_by_default() {
        let config = AppConfig::default();
        assert!(config.captcha.provider.is_empty());
    }

    #[test]
    fn test_spam_enabled_by_default() {
        let config = AppConfig::default();
        assert!(config.spam.enabled);
        assert_eq!(config.spam.threshold, 0.7);
    }

    #[test]
    fn test_load_from_toml_file() {
        // Create a temporary config file
        let mut temp_file = tempfile::NamedTempFile::new().unwrap();
        writeln!(
            temp_file,
            r#"
[site]
name = "Test Forum"
description = "A test forum"
base_url = "https://test.example.com"

[captcha]
provider = "turnstile"
site_key = "test_site_key"
failed_login_threshold = 5

[security]
max_failed_logins = 10
lockout_duration_minutes = 30

[limits]
posts_per_page = 50
"#
        )
        .unwrap();

        let config = AppConfig::load_from_path(temp_file.path().to_str().unwrap()).unwrap();

        assert_eq!(config.site.name, "Test Forum");
        assert_eq!(config.site.base_url, "https://test.example.com");
        assert_eq!(config.captcha.provider, "turnstile");
        assert_eq!(config.captcha.site_key, "test_site_key");
        assert_eq!(config.captcha.failed_login_threshold, 5);
        assert_eq!(config.security.max_failed_logins, 10);
        assert_eq!(config.security.lockout_duration_minutes, 30);
        assert_eq!(config.limits.posts_per_page, 50);
        // Defaults should still apply for unspecified values
        assert_eq!(config.limits.threads_per_page, 20);
    }

    #[test]
    fn test_missing_config_file_uses_defaults() {
        let config = AppConfig::load_from_path("/nonexistent/config.toml").unwrap();
        assert_eq!(config.site.name, "Ruforo");
        assert_eq!(config.captcha.failed_login_threshold, 3);
    }
}
