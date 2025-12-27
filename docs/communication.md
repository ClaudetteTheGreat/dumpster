# Communication & Notifications

This document covers the communication features including notifications, private messaging, chat, and RSS feeds.

## Notification System

### In-App Notifications
- Real-time notifications for user interactions
- Notification bell in header with unread count
- Click to view notification center

### Notification Types
- **Mention** - When someone @mentions you in a post
- **Reply** - When someone replies to your thread
- **Thread Watch** - New posts in watched threads
- **Private Message** - New conversation messages
- **Quote** - When someone quotes your post
- **Moderation Action** - Warnings, bans, or other mod actions

### Notification Preferences
- Per-type configuration for delivery method:
  - In-app notifications (on/off)
  - Email notifications (on/off)
  - Frequency options: Immediate, Hourly digest, Daily digest, Never
- Access preferences at `/notifications/preferences`
- Configure separately for each notification type

### Read/Unread Tracking
- Mark individual notifications as read
- "Mark All Read" button for bulk marking
- Unread count displayed in header

## Private Messaging

### Direct Messages
- Send private messages between users
- Recipients selected by username
- Subject line and message body

### Conversation Threads
- Messages organized into conversation threads
- Reply to continue conversation
- View full message history

### Participant Management
- Multi-user conversations supported
- View all participants in conversation
- Leave conversation option

### Read Status
- Track read/unread status per conversation
- Last read indicator per participant
- Unread conversation count in header

## Thread Watching

### Subscribe to Threads
- Watch button in thread header
- Get notified when someone replies

### Notification Options
- **In-App Only** - Notification in notification center
- **Email Notifications** - Email when new reply posted
  - Toggle with email icon in thread header
  - Only sends to verified email addresses
  - Post author doesn't receive their own notification

### Manage Subscriptions
- View all watched threads at `/watched/threads`
- Unwatch threads individually
- Bulk unwatch option

## Real-Time Chat

### WebSocket Chat
- Real-time messaging via WebSocket
- XenForo compatibility layer available

### Chat Rooms
- Multi-room support
- User activity tracking per room
- Join/leave room functionality

### Message Operations
- **Send** - Post new messages
- **Edit** - Modify your messages (creates revision)
- **Delete** - Soft delete (preserves audit trail)

### User Presence
- See who's online in each room
- User list updates in real-time
- Typing indicators (when enabled)

### Smilie Support
- Configurable emoticon replacement
- Text patterns converted to emoji
- Custom smilie sets supported

### Chat Architecture
The chat system has a pluggable architecture:
- `ChatLayer` trait defines database/storage abstraction
- Default layer uses PostgreSQL
- XF layer provides XenForo MySQL compatibility
- `ChatServer` actor manages WebSocket connections

## RSS Feeds

### Available Feeds
- **Latest Threads** - `/feed.rss` - All forums
- **Per-Forum Feeds** - `/forums/{id}/feed.rss` - Specific forum

### Feed Features
- Standard RSS 2.0 format
- Compatible with all major feed readers
- Automatic `<link rel="alternate">` tags for discovery
- Includes thread title, author, date, and excerpt

### Feed Discovery
RSS-enabled browsers and readers can auto-detect feeds via:
```html
<link rel="alternate" type="application/rss+xml" href="/feed.rss">
```

## @Mentions

### Mention Syntax
- Tag users with `@username` in posts
- Case-insensitive matching

### Autocomplete
- Dropdown appears while typing after @
- Shows matching usernames
- Click or tab to select

### Mention Links
- Mentions render as clickable links
- Links to user profile

### Notification
- Automatic notification to mentioned users
- Skips mentions in code blocks and URLs
- Respects user notification preferences

## Email Notifications

### Supported Email Events
- **Thread Reply** - When someone replies to your thread (configurable)
- **Mention** - When someone @mentions you in a post (configurable)
- **Thread Watch** - New posts in threads you're watching (per-thread toggle)
- **Password Reset** - Password reset request emails
- **Email Verification** - Account verification emails
- **Welcome Email** - Sent after email verification

### Email Templates
All emails include:
- Professional HTML formatting with responsive design
- Plain text fallback for email clients that don't support HTML
- Clear call-to-action buttons
- Unsubscribe instructions

### Email Requirements
- SMTP server configuration required
- Users must have verified email addresses
- Email preference set to "on" for the notification type
- Frequency set to "immediate" (digests not yet implemented)

### Email Configuration
See [Configuration](configuration.md) for SMTP setup.
