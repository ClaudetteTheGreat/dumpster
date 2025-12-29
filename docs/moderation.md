# Moderation Tools

This document covers the moderation features available to forum staff.

## Admin Dashboard

The admin dashboard (`/admin`) provides a centralized control panel for moderators and administrators. The dashboard is **permission-gated** - users only see features they have access to.

### Navigation Link
The "Admin" link appears in the top navigation for users with any of these permissions:
- `admin.settings`
- `admin.user.manage`
- `admin.user.ban`
- `moderate.reports.view`
- `moderate.approval.view`
- `admin.word_filters.view`

### Quick Links (Permission-Gated)
| Link | Required Permission |
|------|---------------------|
| User Bans | `admin.user.ban` |
| IP Bans | `admin.user.ban` |
| Reports | `moderate.reports.view` |
| Word Filters | `admin.word_filters.view` |
| Settings | `admin.settings` |
| Feature Flags | `admin.settings` |
| Users | `admin.user.manage` |
| Approval Queue | `moderate.approval.view` |
| Groups | `admin.permissions.manage` |
| Forum Permissions | `admin.permissions.manage` (via forum page) |

### Dashboard Sections (Permission-Gated)
| Section | Required Permission |
|---------|---------------------|
| Recent Users | `admin.user.manage` |
| Recent Moderation | (always visible) |
| Open Reports | `moderate.reports.view` |
| System Info | `admin.settings` |

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

## Forum-Specific Permissions

Override global permissions on a per-forum basis at `/admin/forums/{id}/permissions`:

### Features
- **Permission Matrix** - Visual grid showing all permissions × all groups
- **Per-Forum Overrides** - Set different permissions for each forum
- **Sub-Forum Inheritance** - Child forums inherit parent permissions unless explicitly overridden
- **Thread Inheritance** - Threads automatically inherit their parent forum's permissions

### Permission Resolution Order
1. Check the specific forum for an explicit override
2. If not found, check parent forum (and continue up the hierarchy)
3. If no override in the chain, fall back to global group permission

### Example Use Cases
- **Private Forums** - Deny `forum.view` for Guests in specific forums
- **Read-Only Archives** - Deny `post.create` and `thread.create` for all groups except admins
- **Staff Forums** - Only allow Moderators and Administrators to view/post
- **Announcement Forums** - Allow viewing but restrict thread creation to staff

### Access
- **Admin Link** - "Permissions" button appears on forum pages for users with `admin.permissions.manage`
- **Route** - `/admin/forums/{id}/permissions`

### Inheritance Behavior
```
General Forum (deny post.create for Guests)
├── Announcements (no override) → inherits deny
│   └── Archive (no override) → inherits deny
└── Discussion (allow post.create) → explicit override
```

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

## Reaction Type Management

Manage reaction types and their reputation values:

- **Admin Panel** - Manage reaction types at `/admin/reaction-types`
- **CRUD Operations:**
  - Create new reaction types with custom emoji
  - Edit existing reaction type properties
  - Enable/disable reaction types without deletion
- **Configurable Properties:**
  - **Name** - Display name for the reaction
  - **Emoji** - Unicode emoji or icon
  - **Display Order** - Control picker ordering
  - **Reputation Value** - Points given to post author (+/- values)
  - **Positive Flag** - Whether reaction is considered positive
  - **Active Status** - Enable/disable the reaction type
- **Reputation Impact** - Each reaction type has a configurable reputation value that affects the post author's reputation score

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
