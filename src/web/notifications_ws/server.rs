//! NotificationServer actor for managing notification WebSocket connections
//!
//! This actor maintains a mapping of user IDs to their active connections
//! and broadcasts notifications to connected users in real-time.

use super::message::{
    BroadcastNotification, Connect, Disconnect, GetConnectionCount, NotificationPush,
};
use actix::prelude::*;
use std::collections::HashMap;

/// Stored connection information
struct UserConnection {
    /// Channel to send messages to this connection
    recipient: Recipient<NotificationPush>,
}

/// NotificationServer manages WebSocket connections for real-time notifications
pub struct NotificationServer {
    /// Connection ID counter
    next_id: usize,
    /// Connection ID -> UserConnection
    connections: HashMap<usize, UserConnection>,
    /// User ID -> Vec<Connection IDs> (user may have multiple tabs/devices)
    user_connections: HashMap<i32, Vec<usize>>,
}

impl NotificationServer {
    pub fn new() -> Self {
        log::info!("NotificationServer starting up.");
        Self {
            next_id: 0,
            connections: HashMap::new(),
            user_connections: HashMap::new(),
        }
    }

    /// Send a message to a specific connection
    fn send_to_connection(&self, conn_id: usize, message: String) {
        if let Some(conn) = self.connections.get(&conn_id) {
            conn.recipient.do_send(NotificationPush(message));
        }
    }

    /// Send a message to all connections for a user
    fn send_to_user(&self, user_id: i32, message: String) {
        if let Some(conn_ids) = self.user_connections.get(&user_id) {
            for conn_id in conn_ids {
                self.send_to_connection(*conn_id, message.clone());
            }
        }
    }
}

impl Default for NotificationServer {
    fn default() -> Self {
        Self::new()
    }
}

impl Actor for NotificationServer {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        ctx.set_mailbox_capacity(64);
        log::info!("NotificationServer started");
    }
}

/// Handle new connections
impl Handler<Connect> for NotificationServer {
    type Result = usize;

    fn handle(&mut self, msg: Connect, _: &mut Context<Self>) -> Self::Result {
        // Generate connection ID
        let conn_id = self.next_id;
        self.next_id += 1;

        // Store connection
        self.connections.insert(
            conn_id,
            UserConnection {
                recipient: msg.addr,
            },
        );

        // Map user to connection
        self.user_connections
            .entry(msg.user_id)
            .or_default()
            .push(conn_id);

        log::debug!(
            "User {} connected with connection ID {} (total connections: {})",
            msg.user_id,
            conn_id,
            self.connections.len()
        );

        conn_id
    }
}

/// Handle disconnections
impl Handler<Disconnect> for NotificationServer {
    type Result = ();

    fn handle(&mut self, msg: Disconnect, _: &mut Context<Self>) {
        // Remove connection
        self.connections.remove(&msg.id);

        // Remove from user_connections
        for conn_ids in self.user_connections.values_mut() {
            conn_ids.retain(|&id| id != msg.id);
        }

        // Clean up empty user entries
        self.user_connections.retain(|_, v| !v.is_empty());

        log::debug!(
            "Connection {} disconnected (total connections: {})",
            msg.id,
            self.connections.len()
        );
    }
}

/// Handle notification broadcasts
impl Handler<BroadcastNotification> for NotificationServer {
    type Result = ();

    fn handle(&mut self, msg: BroadcastNotification, _: &mut Context<Self>) {
        // Serialize notification data
        let json = serde_json::json!({
            "type": "notification",
            "data": msg.notification
        });

        if let Ok(message) = serde_json::to_string(&json) {
            self.send_to_user(msg.user_id, message);
            log::debug!("Broadcasted notification to user {}", msg.user_id);
        }
    }
}

/// Get connection count (for monitoring)
impl Handler<GetConnectionCount> for NotificationServer {
    type Result = usize;

    fn handle(&mut self, _: GetConnectionCount, _: &mut Context<Self>) -> Self::Result {
        self.connections.len()
    }
}

impl Supervised for NotificationServer {
    fn restarting(&mut self, _: &mut Context<NotificationServer>) {
        log::warn!("Restarting the NotificationServer.");
    }
}
