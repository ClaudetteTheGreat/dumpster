//! WebSocket connection actor for notification clients

use super::message::{Connect, Disconnect, NotificationPush};
use super::server::NotificationServer;
use super::{CLIENT_TIMEOUT, HEARTBEAT_INTERVAL};
use actix::*;
use actix_web_actors::ws;
use std::time::Instant;

/// Represents a single WebSocket connection for notifications
pub struct NotificationConnection {
    /// Connection ID (assigned by server)
    pub id: usize,
    /// User ID for this connection
    pub user_id: i32,
    /// Last heartbeat timestamp
    pub hb: Instant,
    /// Address of the notification server
    pub server: Addr<NotificationServer>,
}

impl NotificationConnection {
    pub fn new(user_id: i32, server: Addr<NotificationServer>) -> Self {
        Self {
            id: 0,
            user_id,
            hb: Instant::now(),
            server,
        }
    }

    /// Start heartbeat process
    fn hb(&self, ctx: &mut ws::WebsocketContext<Self>) {
        ctx.run_interval(HEARTBEAT_INTERVAL, |act, ctx| {
            // Check client heartbeats
            if Instant::now().duration_since(act.hb) > CLIENT_TIMEOUT {
                // Heartbeat timed out - disconnect
                log::debug!("Notification connection {} timed out", act.id);
                act.server.do_send(Disconnect { id: act.id });
                ctx.stop();
                return;
            }

            ctx.ping(b"");
        });
    }

    /// Register with server and start heartbeat
    fn start_connection(&self, ctx: &mut ws::WebsocketContext<Self>) {
        // Start heartbeat
        self.hb(ctx);

        // Register with notification server
        self.server
            .send(Connect {
                addr: ctx.address().recipient(),
                user_id: self.user_id,
            })
            .into_actor(self)
            .then(|res, act, ctx| {
                match res {
                    Ok(id) => {
                        act.id = id;
                        log::debug!(
                            "Notification connection established: id={}, user={}",
                            id,
                            act.user_id
                        );
                    }
                    Err(err) => {
                        log::warn!("Failed to register notification connection: {:?}", err);
                        ctx.stop();
                    }
                }
                fut::ready(())
            })
            .wait(ctx);
    }
}

impl Actor for NotificationConnection {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        self.start_connection(ctx);
    }

    fn stopping(&mut self, _: &mut Self::Context) -> Running {
        // Notify server of disconnect
        self.server.do_send(Disconnect { id: self.id });
        Running::Stop
    }
}

/// Handle messages pushed from the notification server
impl Handler<NotificationPush> for NotificationConnection {
    type Result = ();

    fn handle(&mut self, msg: NotificationPush, ctx: &mut Self::Context) {
        ctx.text(msg.0);
    }
}

/// Handle incoming WebSocket messages
impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for NotificationConnection {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        let msg = match msg {
            Err(_) => {
                ctx.stop();
                return;
            }
            Ok(msg) => msg,
        };

        match msg {
            ws::Message::Ping(data) => {
                self.hb = Instant::now();
                ctx.pong(&data);
            }
            ws::Message::Pong(_) => {
                self.hb = Instant::now();
            }
            ws::Message::Text(text) => {
                // Handle client commands if needed
                let text = text.trim();
                if text == "ping" {
                    // Simple ping/pong for keep-alive
                    ctx.text(r#"{"type":"pong"}"#);
                }
                // Notifications are server-push only, so we ignore other messages
            }
            ws::Message::Binary(_) => {
                // Ignore binary messages
            }
            ws::Message::Close(reason) => {
                log::debug!("Notification client disconnecting: {:?}", reason);
                ctx.close(reason);
                ctx.stop();
            }
            ws::Message::Continuation(_) => {
                ctx.stop();
            }
            ws::Message::Nop => (),
        }
    }
}
