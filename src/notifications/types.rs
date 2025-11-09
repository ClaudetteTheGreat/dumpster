//! Notification type definitions

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NotificationType {
    Reply,          // Someone replied to your thread
    Mention,        // You were mentioned in a post
    Quote,          // Your post was quoted
    PrivateMessage, // New private message
    ThreadWatch,    // Update in watched thread
    ModAction,      // Moderation action on your content
}

impl NotificationType {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Reply => "reply",
            Self::Mention => "mention",
            Self::Quote => "quote",
            Self::PrivateMessage => "pm",
            Self::ThreadWatch => "thread_watch",
            Self::ModAction => "mod_action",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "reply" => Some(Self::Reply),
            "mention" => Some(Self::Mention),
            "quote" => Some(Self::Quote),
            "pm" => Some(Self::PrivateMessage),
            "thread_watch" => Some(Self::ThreadWatch),
            "mod_action" => Some(Self::ModAction),
            _ => None,
        }
    }
}
