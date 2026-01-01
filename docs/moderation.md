# Moderation Tools

This document covers the moderation features available to forum staff.

## Admin Dashboard

The admin dashboard (`/admin`) provides a centralized control panel for moderators and administrators. The dashboard is **permission-gated** - users only see features they have access to.

### Access Control
The admin dashboard requires at least one admin or moderation permission to access. Users without any admin permissions receive a 403 Forbidden error.

### Navigation Link
The "Admin" link appears in the top navigation for users with any of these permissions:
- `admin.settings`
- `admin.user.manage`
- `admin.user.ban`
- `admin.permissions.manage`
- `admin.word_filters.view`
- `moderate.reports.view`
- `moderate.approval.view`

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
| Permission Viewer | `admin.settings` |
| Forums | `admin.settings` |
| Reaction Types | `admin.settings` |
| Badges | `admin.settings` |
| Forum Permissions | `admin.permissions.manage` (via forum page) |

### Dashboard Sections (Permission-Gated)
| Section | Required Permission |
|---------|---------------------|
| Statistics Grid | `admin.settings` |
| Recent Users | `admin.user.manage` |
| Recent Moderation | `moderate.reports.view` or `admin.settings` |
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

## Permission Hierarchy Viewer

Visual tool for inspecting effective permissions at `/admin/permissions/hierarchy`:

### Features
- **User Search** - Look up any user by username with autocomplete
  - Type 2+ characters to see suggestions
  - Keyboard navigation (arrow keys, Enter to select)
- **Group View** - Select a group to see its permissions
- **Forum View** - Select a forum to see its permissions and moderators
- **User Information Displayed:**
  - Group memberships with primary group indicated
  - Forum moderator status for each forum (direct and inherited)
  - Effective permissions with source group attribution
- **Group Information Displayed:**
  - Member count and list (first 20 members)
  - All permissions granted to the group
- **Forum Information Displayed:**
  - Forum moderators with source indication:
    - Direct moderators (assigned to this specific forum)
    - Inherited moderators (from parent forums)
  - Parent forum inheritance info
  - Permissions for each group in that forum

### Permission Resolution
Shows the final effective permission after resolving:
1. User's group memberships
2. Permission precedence (Never > Yes > No)
3. Source attribution (which group granted each permission)

### Access
- **Route** - `/admin/permissions/hierarchy`
- **Required Permission** - `admin.settings`
- **Dashboard Link** - "Permission Viewer" in admin quick links

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

### Live Reload
- Permission changes take effect **immediately** without server restart
- Uses global `RwLock` store shared across all workers
- When permissions are saved, `reload_forum_permissions()` updates the cache

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

## Forum Management

Manage forum settings and appearance:

- **Admin Panel** - Manage forums at `/admin/forums`
- **Editable Properties:**
  - **Name** - Display name for the forum
  - **Description** - Brief description shown below forum name
  - **Display Order** - Control forum ordering (lower numbers first)
  - **Parent Forum** - Nest forums under parent forums for hierarchy
- **Custom Icons:**
  - **Default Icon** - Emoji/text shown when forum has no new content
  - **New Content Icon** - Emoji/text shown when forum has unread content
  - **Custom Images** - Upload PNG, GIF, WebP, or SVG images for icons (32x32 or 48x48 recommended)
  - Images take priority over emoji when both are set
  - Separate images for default and new content states
  - File deduplication via BLAKE3 hashing
- **Access** - Link in admin dashboard under "Forums" (requires `admin.settings` permission)

## Reaction Type Management

Manage reaction types and their reputation values:

- **Admin Panel** - Manage reaction types at `/admin/reaction-types`
- **CRUD Operations:**
  - Create new reaction types with custom emoji or images
  - Edit existing reaction type properties
  - Enable/disable reaction types without deletion
- **Configurable Properties:**
  - **Name** - Display name for the reaction
  - **Emoji** - Unicode emoji or icon (fallback)
  - **Custom Image** - Upload PNG, GIF, or WebP image (takes priority over emoji)
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
