# User Interface & Features

This document covers the user-facing features and interface elements.

## Navigation & Discoverability

- **Breadcrumb Navigation** - Hierarchical navigation (Home → Forums → Forum → Thread)
- **Latest Post Navigation** - Quick jump to most recent post from thread header and forum listings
- **Enhanced Pagination** - Previous/Next buttons, current page highlighting, smart ellipsis (1 2 3 ... 8 [9] 10 ... 15)
- **Jump to Post** - Direct linking to specific posts with `/threads/{id}/post-{post_id}`
- **New Posts Feed** - `/recent/posts` shows latest posts across all forums with navigation link in header
- **New Threads Feed** - `/recent/threads` shows latest threads across all forums

## Forum Features

- **Forum Statistics** - Thread and post counts displayed on forum index
- **Forum Rules Display** - Optional forum-specific rules displayed at the top of each forum in a highlighted box
- **Forum Moderators** - Display moderators assigned to each forum with profile links
- **Custom Forum Icons** - Customize forum folder icons
  - Emoji/text icons for default (no new posts) and new content states
  - Upload custom PNG, GIF, WebP, or SVG images for icons
  - Images take priority over emoji fallback
  - Separate images for unread vs read forum states
  - Managed via admin panel at `/admin/forums`
- **Sub-Forums** - Hierarchical forum structure with parent/child relationships
  - Forums can be nested under parent forums
  - Sub-forums displayed with visual indentation in forum index
  - Breadcrumb navigation includes full parent chain
  - Sub-forums section displayed within parent forum view
- **Read Tracking** - Track read/unread status for forums and threads
  - Unread indicators (folder icon, blue border) for forums with new posts
  - "Mark as Read" button to mark individual forums as read
  - "Mark All Read" button to mark all forums as read at once
  - Jump to first unread post via `/threads/{id}/unread`
  - "Unread" link in thread listings for quick navigation
- **Thread Status Badges** - Visual indicators for pinned and locked threads
- **Thread Metadata** - Post count and view count displayed in thread headers
- **Latest Activity** - Timestamp and link to latest post in forum thread listings

## Thread Features

- **Thread Prefixes** - Categorize threads with prefixes like [SOLVED], [QUESTION], [DISCUSSION] displayed as badges
- **Thread Tags** - Add tags during thread creation for categorization and discoverability
  - Comma-separated tag input with auto-slug generation
  - Colored badge display in listings and thread views
  - Forum-specific or global tag scope
  - Automatic tag use count tracking
  - Filter threads by tag via `?tag=slug` query parameter
  - Active filter indicator with clear button
- **Watch Threads** - Subscribe to threads for notifications on new posts
- **Deleted Post Handling** - Placeholder display for deleted posts with deletion timestamp
- **Post History** - Track post edits with revision history
- **Attachments** - File upload support with S3 storage integration
- **Thread Polls** - Create polls when starting threads
  - Single or multiple choice voting with configurable max choices
  - Optional vote changing after initial vote
  - Results visibility before/after voting
  - Optional poll closing date
  - Real-time vote count display with percentage bars
  - Full dark mode support
- **Similar Threads** - Discover related content based on shared tags
  - Displays up to 5 similar threads at the bottom of thread view
  - Ranked by number of matching tags, then recency
  - Shows post count and number of tags in common

## Post Features

- **Post Reactions** - React to posts with emoji reactions (like, thanks, funny, informative, agree, disagree)
  - Toggle reactions on/off with single click
  - Real-time reaction count updates
  - Visual indication of user's own reactions
  - Database-backed with automatic count triggers
  - Reputation system: reactions affect post author's reputation score
  - Admin-configurable reputation values per reaction type
  - Voting restrictions: cannot react to own posts, minimum post count required
- **Quote Reply** - Click Quote button on any post to insert quoted content into reply
  - Inserts `[quote=username]content[/quote]` BBCode
  - Scrolls to and focuses the reply textarea
- **Multi-Quote** - Queue multiple posts to quote at once
  - Click +Quote to add posts to queue (persists across pages via localStorage)
  - Floating indicator shows number of selected quotes
  - "Insert Quotes" inserts all queued quotes at once
  - "Clear" removes all queued quotes
  - Button toggles to -Quote when post is in queue
- **Draft Auto-Save** - Prevent data loss with automatic draft saving
  - Automatically saves post content to localStorage every 2 seconds
  - Restores draft when returning to page (with "Draft restored" indicator)
  - Clears draft on successful form submission
  - Works for thread replies, new threads, and conversations
  - Drafts expire after 7 days
  - "Clear draft" button to discard saved content
- **Quick Reply** - Reply button in thread header for fast access
  - Smooth scroll to reply form
  - Auto-focus on textarea
- **Report Post** - Report posts for moderation review
  - Modal dialog with reason selection (spam, harassment, off-topic, illegal content, misinformation, other)
  - Optional details field (required for "Other" reason)
  - Duplicate report prevention
  - Admin panel for reviewing and managing reports at `/admin/reports`
- **@Mentions** - Tag users in posts with `@username`
  - Autocomplete dropdown while typing
  - Clickable mention links to user profiles
  - Automatic notifications to mentioned users
  - Skips mentions in code blocks and URLs

## BBCode Formatting

Rich text formatting for posts:

- **Basic Formatting**: Bold `[b]`, Italic `[i]`, Underline `[u]`, Strikethrough `[s]`, Color `[color=red]`
- **Text Styling**: Size `[size=8-36]`, Font `[font=arial]` (whitelisted fonts only)
- **Text Alignment**: Center `[center]`, Left `[left]`, Right `[right]`
- **Lists**: Unordered `[list][*]`, Numbered `[list=1][*]`, Alphabetic `[list=a][*]` with nesting support
- **Quotes**: Basic `[quote]` and attributed `[quote=username]` with "username said:" display
- **Spoilers**: Collapsible content with `[spoiler]` and custom titles `[spoiler=title]`
- **Code**: Preformatted code blocks `[code]` with preserved whitespace and syntax highlighting
  - Language-specific highlighting with `[code=language]` (e.g., `[code=rust]`, `[code=javascript]`)
  - 50+ languages supported with highlight.js (client-side)
  - Common aliases: js→javascript, py→python, ts→typescript, sh→bash
  - GitHub-inspired color theme with automatic dark mode support
- **Images**: `[img]` with optional dimensions `[img=200x150]` or width-only `[img=200]`, Links `[url]` with automatic URL detection
- **Video Embeds**: `[video]` for YouTube, Vimeo, and direct video files (.mp4, .webm, .ogg)
  - YouTube privacy-enhanced embeds via youtube-nocookie.com
  - Responsive 16:9 aspect ratio for embedded players
  - `[youtube]videoId[/youtube]` shorthand for YouTube videos
- **Audio Embeds**: `[audio]` for audio files (.mp3, .ogg, .wav, .flac, .m4a)
- **Media Auto-Detect**: `[media]` automatically detects and embeds YouTube, Vimeo, video, or audio based on URL
- **Tables**: `[table][tr][td]...[/td][/tr][/table]` with header support `[th]`
  - Auto-closing cells when opening new ones
  - Validation ensures proper nesting (cells inside rows, rows inside tables)
  - Responsive styling with dark mode support
- **URL Unfurl**: Rich link previews with `[url unfurl]https://example.com[/url]`
  - Extracts Open Graph metadata (title, description, image)
  - Shows site favicon and site name
  - 24-hour server-side cache for performance
  - Async JavaScript hydration for fast page loads
  - Responsive card layout with dark mode support
- **Security**: HTML entity sanitization, XSS prevention at tokenizer level, dimension validation (max 2000px)

### BBCode Toolbar

Visual editor toolbar for post formatting:
- One-click buttons for bold, italic, underline, strikethrough
- Link and image insertion with prompts
- Quote, code, and spoiler blocks
- List creation with automatic item formatting
- Quick color buttons (red, green, blue) and custom color picker
- Text size controls (small/large)

### Post Preview

Server-side BBCode preview before posting:
- Toggle between edit and preview modes
- Real-time rendering via `/api/bbcode/preview` endpoint
- Shows rendered HTML exactly as it will appear
- Dark mode support

## Keyboard Shortcuts

- **Post Navigation**: `j`/`k` to navigate between posts (vim-style)
- **Go To Navigation**: `g` then `h` (home), `f` (forums), `n` (new posts), `w` (watched threads)
- **Quick Actions**: `r` (reply), `q` (quote focused post), `/` (focus search)
- **Help & Escape**: `?` (show shortcuts help), `Escape` (close modals, unfocus)
- **Smart Detection**: Shortcuts disabled when typing in text fields
- **Help Modal**: Press `?` to view all available shortcuts

## User Information Display

- **Thread Starter Badge** - "OP" badge displayed next to original poster's name
- **User Post Counts** - Total post count shown in message sidebar
- **Join Date Display** - User registration date shown as "Joined: Mon YYYY"
- **User Avatars** - Avatar display with multiple size options (S/M/L)
  - Drag-and-drop upload with image preview
  - Client-side file type and size validation
  - Support for JPEG, PNG, GIF, and WebP formats
- **Custom Title** - User-defined title displayed under username in posts (100 character limit)
- **Online Status** - Track and display which users are currently active
  - Users shown as online if active within the last 15 minutes
  - Online user count and list displayed on forum index page
  - Rate-limited activity tracking (updates at most once per 60 seconds)
  - Privacy setting to hide online status from other users
  - Hidden users excluded from online counts and listings
- **Reputation Score** - Aggregate score based on reactions received
  - Displayed in post sidebar and member profile
  - Color-coded: green for positive, red for negative
  - Updated automatically via database triggers when reactions change

## Responsive Design

- All UI components are mobile-friendly with appropriate breakpoints
- Statistics and metadata hidden on mobile for cleaner layout
- Touch-friendly button sizes and spacing

## Accessibility

- **Skip Links** - Keyboard users can skip to main content
- **ARIA Labels** - Screen reader support for navigation, forms, and interactive elements
- **Semantic Roles** - Proper banner, main, and contentinfo roles
- **Pagination** - Accessible page navigation with current page indication
- **Form Accessibility** - Proper labels and autocomplete attributes

## Private Messaging

Private conversations between users:

- **Inbox** - View all active conversations with unread indicators
- **Multi-Participant Conversations** - Create conversations with multiple users
- **Read Tracking** - Per-user read/unread status for each conversation
- **Message History** - Full conversation history with pagination
- **Leave Conversation** - Remove yourself from a conversation
  - Confirmation dialog before leaving
  - Conversation auto-deleted when all participants leave
- **Archive Conversations** - Hide conversations from inbox without deleting
  - Per-user archive status (doesn't affect other participants)
  - Archived conversations page at `/conversations/archived`
  - Unarchive to restore to inbox
- **Notifications** - In-app alerts for new messages
- **Unread Badge** - Message count displayed in navigation header

## User Preferences & Customization

- **Dark Mode** - Toggle between light, dark, and auto (system preference) themes
  - Persistent theme preference stored per user
  - Real-time theme switching without page reload
  - Comprehensive dark mode styling for all UI components
  - Auto mode respects operating system dark mode preference
- **Posts Per Page** - Configurable pagination (10, 25, 50, or 100 posts per page)
- **Show Online Status** - Privacy toggle to hide/show online presence to other users
- **Character Counter** - Real-time character counting for post/thread creation
  - Visual feedback (green/yellow/red) based on remaining characters
  - Automatic limit detection (50,000 for users, 100,000 for moderators)
  - Form submission prevention when over limit
  - Client-side validation before submission
