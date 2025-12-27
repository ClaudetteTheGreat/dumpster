//! Message types for the notification WebSocket system

use actix::prelude::*;
use serde::Serialize;

/// New notification WebSocket connection
pub struct Connect {
    /// Channel to send messages back to this connection
    pub addr: Recipient<NotificationPush>,
    /// User ID for this connection
    pub user_id: i32,
}

impl Message for Connect {
    /// Returns connection ID
    type Result = usize;
}

/// Disconnect message
pub struct Disconnect {
    /// Connection ID
    pub id: usize,
}

impl Message for Disconnect {
    type Result = ();
}

/// Push a notification to a specific user
#[derive(Clone)]
pub struct BroadcastNotification {
    /// Target user ID
    pub user_id: i32,
    /// Notification data
    pub notification: NotificationData,
}

impl Message for BroadcastNotification {
    type Result = ();
}

/// Notification data to send to client
#[derive(Clone, Serialize)]
pub struct NotificationData {
    pub id: i32,
    pub notification_type: String,
    pub title: String,
    pub message: String,
    pub url: Option<String>,
    pub created_at: String,
}

/// Server -> Client push message
pub struct NotificationPush(pub String);

impl Message for NotificationPush {
    type Result = ();
}

/// Get count of connected users (for debugging)
pub struct GetConnectionCount;

impl Message for GetConnectionCount {
    type Result = usize;
}
