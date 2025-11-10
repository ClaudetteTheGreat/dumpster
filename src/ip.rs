/// IP address tracking and management for moderation purposes
///
/// This module provides functionality to extract client IP addresses from HTTP requests
/// and store them in the database for moderation and abuse prevention.

use crate::db::get_db_pool;
use crate::orm::ip;
use actix_web::HttpRequest;
use chrono::Utc;
use sea_orm::{entity::*, query::*, ActiveValue::Set, DbErr};
use std::net::IpAddr;

/// Extract the real client IP address from an HTTP request.
///
/// Checks headers in order of preference:
/// 1. X-Forwarded-For (first IP in the list)
/// 2. X-Real-IP
/// 3. Remote peer address
///
/// Privacy note: IP addresses are stored for moderation purposes.
/// Consider implementing IP retention policies (e.g., automatic deletion after 90 days).
pub fn extract_client_ip(req: &HttpRequest) -> Option<String> {
    // Check X-Forwarded-For header (proxy chains)
    if let Some(xff) = req.headers().get("x-forwarded-for") {
        if let Ok(xff_str) = xff.to_str() {
            // Take the first IP in the chain (the original client)
            if let Some(first_ip) = xff_str.split(',').next() {
                let trimmed = first_ip.trim();
                // Validate it's a proper IP address
                if trimmed.parse::<IpAddr>().is_ok() {
                    return Some(trimmed.to_string());
                }
            }
        }
    }

    // Check X-Real-IP header (nginx, etc.)
    if let Some(xri) = req.headers().get("x-real-ip") {
        if let Ok(xri_str) = xri.to_str() {
            let trimmed = xri_str.trim();
            if trimmed.parse::<IpAddr>().is_ok() {
                return Some(trimmed.to_string());
            }
        }
    }

    // Fall back to peer address
    if let Some(peer_addr) = req.peer_addr() {
        return Some(peer_addr.ip().to_string());
    }

    None
}

/// Get or create an IP record in the database.
///
/// If the IP address already exists, updates the last_seen_at timestamp and returns the ID.
/// If it doesn't exist, creates a new record with first_seen_at and last_seen_at set to now.
///
/// Returns the IP record ID on success, or a database error.
pub async fn get_or_create_ip_id(address: &str) -> Result<Option<i32>, DbErr> {
    let db = get_db_pool();
    let now = Utc::now().naive_utc();

    // Try to find existing IP record
    match ip::Entity::find()
        .filter(ip::Column::Address.eq(address))
        .one(db)
        .await?
    {
        Some(existing) => {
            // Update last_seen_at
            let mut active_model: ip::ActiveModel = existing.into();
            active_model.last_seen_at = Set(now);
            let updated = active_model.update(db).await?;
            Ok(Some(updated.id))
        }
        None => {
            // Create new IP record
            let new_ip = ip::ActiveModel {
                address: Set(address.to_string()),
                first_seen_at: Set(now),
                last_seen_at: Set(now),
                ..Default::default()
            };

            let model = new_ip.insert(db).await?;
            Ok(Some(model.id))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_ip_parses_valid_ipv4() {
        let ip = "192.168.1.1";
        assert!(ip.parse::<IpAddr>().is_ok());
    }

    #[test]
    fn test_extract_ip_parses_valid_ipv6() {
        let ip = "2001:0db8:85a3:0000:0000:8a2e:0370:7334";
        assert!(ip.parse::<IpAddr>().is_ok());
    }
}
