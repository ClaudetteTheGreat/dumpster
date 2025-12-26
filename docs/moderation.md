# Moderation Tools

This document covers the moderation features available to forum staff.

## Thread Moderation

- **Thread Moderation UI** - Lock/Unlock and Pin/Unpin controls for moderators
- **Thread Move** - Move threads between forums
  - Forum selection dropdown with all available destinations
  - Optional reason field for moderation logs
  - Metadata logged includes source and destination forum IDs
- **Thread Merge** - Combine threads together
  - Moves all posts from source thread to target thread
  - Recalculates post counts and first/last post references
  - Source thread marked as merged with link to target
  - Merged threads hidden from forum listings

## Mass Moderation Actions

Bulk operations on users from the admin user management panel:

- **Checkbox Selection** - Select individual users or use "Select All"
- **Available Actions:**
  - **Ban** - Ban selected users with configurable duration
  - **Unban** - Remove bans from selected users
  - **Verify Email** - Mark emails as verified
  - **Approve** - Approve pending user registrations
  - **Delete** - Permanently delete user accounts
- **Confirmation Dialogs** - Require confirmation before executing bulk actions
- **Ban Duration** - Configurable duration for mass bans (days)

## User Warning System

Progressive discipline with point-based warnings:

- **Warning Points** - Configurable points per warning (1-10)
- **Warning Expiration** - Options: 30, 60, 90, 180, 365 days, or permanent
- **Auto-Ban Threshold** - Automatic ban when warning points exceed threshold (configurable)
- **Warning History** - View complete warning history per user
- **Warning Details** - Reason, points, expiration date, and issuing moderator

## User Approval Queue

Manual approval workflow for new registrations:

- **Approval Queue** - View pending users at `/admin/approval-queue`
- **Actions:**
  - **Approve** - Activate the user account
  - **Reject** - Reject with optional reason
- **Configuration** - Enable/disable via `require_user_approval` setting

## Moderator Notes

Private notes visible only to staff:

- **Per-User Notes** - Notes attached to user profiles
- **Operations:** Add, view, and delete notes
- **Staff-Only Visibility** - Notes hidden from regular users

## Report System

User-submitted reports for content moderation:

- **Report Modal** - Modal dialog with reason selection
- **Report Reasons:**
  - Spam
  - Harassment
  - Off-topic
  - Illegal content
  - Misinformation
  - Other (requires details)
- **Admin Panel** - Review and manage reports at `/admin/reports`
- **Duplicate Prevention** - Users cannot report the same content twice

## User Bans

- **Temporary Bans** - Ban for specified duration
- **Permanent Bans** - No expiration
- **Ban Reasons** - Required reason for audit trail
- **Ban Management** - View and manage bans at `/admin/bans`

## IP Bans

- **IP Address Bans** - Block specific IP addresses
- **Range Support** - Ban IP ranges (CIDR notation)
- **Ban Management** - View and manage IP bans at `/admin/ip-bans`

## Permission Groups

Create and manage user groups at `/admin/groups`:

### Custom Permission Groups
- Create custom groups with specific permissions
- Edit group names and permissions
- Delete custom groups (system groups protected)
- View member count for each group

### System Groups (Built-in, cannot be deleted)
- **Guests** - Read-only access for unauthenticated users
- **Registered Users** - Basic permissions for logged-in users
- **Moderators** - Content moderation permissions
- **Administrators** - Full system access

### Permission Values
- **Yes** - Grant the permission
- **No** - Deny the permission
- **Never** - Permanent deny (cannot be overridden by other groups)
- **Default** - Inherit from other groups

## Word Filters

Admin-configurable content filters for automatic moderation:

- **Filter Actions:**
  - **Replace** - Substitute matched words with alternatives (e.g., "Solana" -> "Salona")
  - **Block** - Reject content containing specific words entirely
  - **Flag** - Allow content but mark it for moderator review
- **Matching Options:**
  - Regular expression support for complex patterns
  - Case-sensitive or case-insensitive matching
  - Whole-word only or partial matching within words
  - Enable/disable individual filters without deletion
- **Case Preservation** - Replacements preserve original case
- **Admin Panel** - Full CRUD interface at `/admin/word-filters`
- **Integration** - Applied to thread creation (title and content) and post replies
- **Efficient Caching** - Compiled regex patterns cached in memory

## Moderation Logging

All moderation actions are logged:

- **Logged Actions:**
  - Thread lock/unlock/pin/unpin
  - Thread move/merge
  - User ban/unban
  - User warnings issued
  - Content deletion
- **Log Contents:**
  - Action type
  - Target (user, thread, post)
  - Moderator who performed action
  - Timestamp
  - Reason (when provided)
- **Access** - Moderation logs viewable at `/admin/mod-log`

## Security

- **Permission-Based Display** - Tools only visible to users with appropriate permissions
- **CSRF Protection** - All moderation operations protected against CSRF attacks
- **IP Tracking** - IP addresses logged for all posts and threads
- **Audit Trail** - Comprehensive logging of all moderation actions
