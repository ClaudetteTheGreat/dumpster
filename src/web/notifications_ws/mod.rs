//! Real-time notification WebSocket system
//!
//! This module provides WebSocket connections for pushing notifications
//! to connected users in real-time.
//!
//! ## Architecture
//!
//! - `NotificationServer` actor maintains user connections
//! - `NotificationConnection` actor handles individual WebSocket connections
//! - The dispatcher calls `broadcast_notification()` when creating notifications
//!
//! ## Usage
//!
//! 1. Client connects to `/notifications.ws`
//! 2. Server registers connection and maps to user_id
//! 3. When notifications are created, they're pushed to connected clients
//! 4. Client receives JSON messages with notification data

pub mod connection;
pub mod message;
pub mod server;

use crate::middleware::ClientCtx;
use actix::Addr;
use actix_web::{get, web, Error, HttpRequest, HttpResponse};
use actix_web_actors::ws;
use once_cell::sync::OnceCell;
use std::time::Duration;

pub use message::{BroadcastNotification, NotificationData};
pub use server::NotificationServer;

/// Global notification server instance
static NOTIFICATION_SERVER: OnceCell<Addr<NotificationServer>> = OnceCell::new();

/// Initialize the global notification server
pub fn init_notification_server(server: Addr<NotificationServer>) {
    NOTIFICATION_SERVER
        .set(server)
        .expect("NotificationServer already initialized");
}

/// Get the global notification server address
pub fn get_notification_server() -> Option<&'static Addr<NotificationServer>> {
    NOTIFICATION_SERVER.get()
}

/// Heartbeat interval - send ping every 5 seconds
pub const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);

/// Client timeout - disconnect if no response for 30 seconds
pub const CLIENT_TIMEOUT: Duration = Duration::from_secs(30);

/// Configure notification WebSocket routes
pub fn configure(conf: &mut web::ServiceConfig) {
    conf.service(notifications_ws);
}

/// WebSocket endpoint for real-time notifications
///
/// GET /notifications.ws
///
/// Requires authentication. Connects the user to the notification
/// broadcast system for real-time notification delivery.
#[get("/notifications.ws")]
pub async fn notifications_ws(
    req: HttpRequest,
    stream: web::Payload,
    client: ClientCtx,
    server: web::Data<Addr<NotificationServer>>,
) -> Result<HttpResponse, Error> {
    // Require login
    let user_id = client.require_login()?;

    log::debug!("User {} connecting to notification WebSocket", user_id);

    // Create connection actor
    let connection = connection::NotificationConnection::new(user_id, server.get_ref().clone());

    // Start WebSocket
    ws::start(connection, &req, stream)
}

/// Broadcast a notification to a user via the notification server
///
/// This function is called by the notification dispatcher when a new
/// notification is created. If the user is connected, they'll receive
/// the notification in real-time.
pub async fn broadcast_notification(
    server: &Addr<NotificationServer>,
    user_id: i32,
    notification_id: i32,
    notification_type: &str,
    title: &str,
    message_text: &str,
    url: Option<&str>,
) {
    let notification = NotificationData {
        id: notification_id,
        notification_type: notification_type.to_string(),
        title: title.to_string(),
        message: message_text.to_string(),
        url: url.map(|s| s.to_string()),
        created_at: chrono::Utc::now().to_rfc3339(),
    };

    server.do_send(BroadcastNotification {
        user_id,
        notification,
    });
}
