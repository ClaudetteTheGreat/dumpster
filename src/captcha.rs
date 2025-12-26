//! CAPTCHA verification module
//!
//! Supports hCaptcha and Cloudflare Turnstile for bot protection.
//!
//! Configuration priority (highest to lowest):
//! 1. Environment variables (RUFORO_CAPTCHA_* or legacy CAPTCHA_*)
//! 2. Config file (config.toml)
//! 3. Default (disabled)
//!
//! Required settings:
//! - provider: "hcaptcha" or "turnstile" (empty to disable)
//! - site_key: Public site key for frontend
//! - secret_key: Secret key for verification (use env var for security)

use crate::app_config;
use once_cell::sync::Lazy;
use serde::Deserialize;

/// CAPTCHA provider type
#[derive(Debug, Clone, PartialEq)]
pub enum CaptchaProvider {
    HCaptcha,
    Turnstile,
    Disabled,
}

/// CAPTCHA configuration
pub struct CaptchaConfig {
    pub provider: CaptchaProvider,
    pub site_key: String,
    pub secret_key: String,
    pub failed_login_threshold: u32,
}

impl CaptchaConfig {
    /// Check if CAPTCHA is enabled
    pub fn is_enabled(&self) -> bool {
        self.provider != CaptchaProvider::Disabled
    }
}

/// Global CAPTCHA configuration
pub static CAPTCHA_CONFIG: Lazy<CaptchaConfig> = Lazy::new(|| {
    // Load from app_config (which already merges file + env vars)
    let config = app_config::captcha();

    // Also check legacy env vars for backward compatibility
    let provider_str = if !config.provider.is_empty() {
        config.provider.clone()
    } else {
        std::env::var("CAPTCHA_PROVIDER").unwrap_or_default()
    };

    let provider = match provider_str.to_lowercase().as_str() {
        "hcaptcha" => CaptchaProvider::HCaptcha,
        "turnstile" => CaptchaProvider::Turnstile,
        _ => CaptchaProvider::Disabled,
    };

    // Site key: prefer config, fall back to legacy env var
    let site_key = if !config.site_key.is_empty() {
        config.site_key.clone()
    } else {
        std::env::var("CAPTCHA_SITE_KEY").unwrap_or_default()
    };

    // Secret key: prefer config (from env var via config crate), fall back to legacy env var
    let secret_key = if !config.secret_key.is_empty() {
        config.secret_key.clone()
    } else {
        std::env::var("CAPTCHA_SECRET_KEY").unwrap_or_default()
    };

    let failed_login_threshold = config.failed_login_threshold;

    if provider != CaptchaProvider::Disabled && (site_key.is_empty() || secret_key.is_empty()) {
        log::warn!(
            "CAPTCHA provider set to {:?} but site_key or secret_key is missing. Disabling CAPTCHA.",
            provider
        );
        return CaptchaConfig {
            provider: CaptchaProvider::Disabled,
            site_key: String::new(),
            secret_key: String::new(),
            failed_login_threshold,
        };
    }

    if provider != CaptchaProvider::Disabled {
        log::info!("CAPTCHA enabled with provider: {:?}", provider);
    }

    CaptchaConfig {
        provider,
        site_key,
        secret_key,
        failed_login_threshold,
    }
});

/// Response from CAPTCHA verification API
#[derive(Debug, Deserialize)]
struct VerifyResponse {
    success: bool,
    #[serde(default)]
    #[serde(rename = "error-codes")]
    error_codes: Vec<String>,
}

/// CAPTCHA verification error
#[derive(Debug)]
pub enum CaptchaError {
    /// CAPTCHA is not configured
    NotConfigured,
    /// Network error during verification
    NetworkError(String),
    /// Verification failed
    VerificationFailed(Vec<String>),
    /// Invalid response token
    InvalidToken,
}

impl std::fmt::Display for CaptchaError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CaptchaError::NotConfigured => write!(f, "CAPTCHA is not configured"),
            CaptchaError::NetworkError(e) => write!(f, "CAPTCHA verification network error: {}", e),
            CaptchaError::VerificationFailed(codes) => {
                write!(f, "CAPTCHA verification failed: {:?}", codes)
            }
            CaptchaError::InvalidToken => write!(f, "Invalid CAPTCHA token"),
        }
    }
}

impl std::error::Error for CaptchaError {}

/// Check if CAPTCHA is enabled
pub fn is_enabled() -> bool {
    CAPTCHA_CONFIG.is_enabled()
}

/// Get the site key for frontend use
pub fn get_site_key() -> Option<&'static str> {
    if CAPTCHA_CONFIG.is_enabled() {
        Some(&CAPTCHA_CONFIG.site_key)
    } else {
        None
    }
}

/// Get the CAPTCHA provider name for frontend
pub fn get_provider_name() -> Option<&'static str> {
    match CAPTCHA_CONFIG.provider {
        CaptchaProvider::HCaptcha => Some("hcaptcha"),
        CaptchaProvider::Turnstile => Some("turnstile"),
        CaptchaProvider::Disabled => None,
    }
}

/// Verify a CAPTCHA response token
///
/// # Arguments
/// * `response_token` - The token from the frontend CAPTCHA widget
/// * `remote_ip` - Optional client IP address for additional verification
///
/// # Returns
/// * `Ok(())` if verification succeeds
/// * `Err(CaptchaError)` if verification fails
pub async fn verify(response_token: &str, remote_ip: Option<&str>) -> Result<(), CaptchaError> {
    if !CAPTCHA_CONFIG.is_enabled() {
        return Err(CaptchaError::NotConfigured);
    }

    if response_token.is_empty() {
        return Err(CaptchaError::InvalidToken);
    }

    let (verify_url, response_field) = match CAPTCHA_CONFIG.provider {
        CaptchaProvider::HCaptcha => ("https://hcaptcha.com/siteverify", "response"),
        CaptchaProvider::Turnstile => {
            ("https://challenges.cloudflare.com/turnstile/v0/siteverify", "response")
        }
        CaptchaProvider::Disabled => return Err(CaptchaError::NotConfigured),
    };

    let client = reqwest::Client::new();

    let mut params = vec![
        ("secret", CAPTCHA_CONFIG.secret_key.as_str()),
        (response_field, response_token),
    ];

    if let Some(ip) = remote_ip {
        params.push(("remoteip", ip));
    }

    let response = client
        .post(verify_url)
        .form(&params)
        .send()
        .await
        .map_err(|e| CaptchaError::NetworkError(e.to_string()))?;

    let verify_response: VerifyResponse = response
        .json()
        .await
        .map_err(|e| CaptchaError::NetworkError(e.to_string()))?;

    if verify_response.success {
        Ok(())
    } else {
        log::warn!(
            "CAPTCHA verification failed: {:?}",
            verify_response.error_codes
        );
        Err(CaptchaError::VerificationFailed(
            verify_response.error_codes,
        ))
    }
}

/// Check if CAPTCHA should be required for login based on failed attempts
///
/// Returns true if the user has exceeded the threshold for failed login attempts
pub fn should_require_for_login(failed_attempts: u32) -> bool {
    CAPTCHA_CONFIG.is_enabled() && failed_attempts >= CAPTCHA_CONFIG.failed_login_threshold
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_enabled_when_disabled() {
        // By default in tests, CAPTCHA should be disabled
        // This test validates the default state
        assert!(!is_enabled() || std::env::var("CAPTCHA_PROVIDER").is_ok());
    }

    #[test]
    fn test_get_provider_name() {
        match CAPTCHA_CONFIG.provider {
            CaptchaProvider::HCaptcha => assert_eq!(get_provider_name(), Some("hcaptcha")),
            CaptchaProvider::Turnstile => assert_eq!(get_provider_name(), Some("turnstile")),
            CaptchaProvider::Disabled => assert_eq!(get_provider_name(), None),
        }
    }

    #[test]
    fn test_should_require_for_login() {
        if is_enabled() {
            assert!(!should_require_for_login(0));
            assert!(!should_require_for_login(2));
            assert!(should_require_for_login(3));
            assert!(should_require_for_login(5));
        } else {
            // When disabled, should never require
            assert!(!should_require_for_login(100));
        }
    }

    #[actix_rt::test]
    async fn test_verify_empty_token() {
        if is_enabled() {
            let result = verify("", None).await;
            assert!(matches!(result, Err(CaptchaError::InvalidToken)));
        }
    }
}
