/**
 * Keyboard shortcuts for forum navigation and actions
 * Press ? to show available shortcuts
 */

(function() {
    'use strict';

    // Track if we're waiting for a second key in a sequence (like 'g' then 'h')
    let pendingPrefix = null;
    let pendingTimeout = null;
    const SEQUENCE_TIMEOUT = 1000; // 1 second to complete sequence

    // Current post index for j/k navigation
    let currentPostIndex = -1;

    /**
     * Check if user is typing in an input field
     */
    function isTyping() {
        const active = document.activeElement;
        if (!active) return false;

        const tagName = active.tagName.toLowerCase();
        if (tagName === 'input' || tagName === 'textarea' || tagName === 'select') {
            return true;
        }

        // Check for contenteditable
        if (active.isContentEditable) {
            return true;
        }

        return false;
    }

    /**
     * Get all message elements on the page
     */
    function getPosts() {
        return Array.from(document.querySelectorAll('.message:not(.message--deleted)'));
    }

    /**
     * Scroll to and highlight a post
     */
    function focusPost(index) {
        const posts = getPosts();
        if (posts.length === 0) return;

        // Clamp index
        index = Math.max(0, Math.min(index, posts.length - 1));
        currentPostIndex = index;

        // Remove highlight from all posts
        posts.forEach(p => p.classList.remove('post-focused'));

        // Add highlight to current post
        const post = posts[index];
        post.classList.add('post-focused');

        // Scroll into view
        post.scrollIntoView({ behavior: 'smooth', block: 'center' });
    }

    /**
     * Navigate to next post
     */
    function nextPost() {
        const posts = getPosts();
        if (posts.length === 0) return;

        if (currentPostIndex < 0) {
            // Find first visible post
            currentPostIndex = 0;
        } else {
            currentPostIndex = Math.min(currentPostIndex + 1, posts.length - 1);
        }

        focusPost(currentPostIndex);
    }

    /**
     * Navigate to previous post
     */
    function prevPost() {
        const posts = getPosts();
        if (posts.length === 0) return;

        if (currentPostIndex < 0) {
            currentPostIndex = 0;
        } else {
            currentPostIndex = Math.max(currentPostIndex - 1, 0);
        }

        focusPost(currentPostIndex);
    }

    /**
     * Focus the reply textarea
     */
    function focusReply() {
        const textarea = document.querySelector('form[action*="post-reply"] textarea[name="content"]');
        if (textarea) {
            textarea.scrollIntoView({ behavior: 'smooth', block: 'center' });
            setTimeout(() => textarea.focus(), 300);
            return true;
        }
        return false;
    }

    /**
     * Quote the currently focused post
     */
    function quoteCurrentPost() {
        const posts = getPosts();
        if (currentPostIndex >= 0 && currentPostIndex < posts.length) {
            const post = posts[currentPostIndex];
            const quoteBtn = post.querySelector('.quote-btn');
            if (quoteBtn) {
                quoteBtn.click();
                return true;
            }
        }
        return false;
    }

    /**
     * Focus the search input
     */
    function focusSearch() {
        const searchInput = document.querySelector('input[name="q"], input[type="search"], .search-input');
        if (searchInput) {
            searchInput.focus();
            return true;
        }
        // Try to find search in header
        const headerSearch = document.querySelector('header input');
        if (headerSearch) {
            headerSearch.focus();
            return true;
        }
        return false;
    }

    /**
     * Navigate to a URL
     */
    function navigateTo(path) {
        window.location.href = path;
    }

    /**
     * Show keyboard shortcuts help modal
     */
    function showHelp() {
        // Remove existing modal if present
        const existing = document.getElementById('keyboard-help-modal');
        if (existing) {
            existing.remove();
            return;
        }

        const modal = document.createElement('div');
        modal.id = 'keyboard-help-modal';
        modal.className = 'keyboard-help-modal';
        modal.innerHTML = `
            <div class="keyboard-help-overlay"></div>
            <div class="keyboard-help-content">
                <div class="keyboard-help-header">
                    <h3>Keyboard Shortcuts</h3>
                    <button type="button" class="keyboard-help-close">&times;</button>
                </div>
                <div class="keyboard-help-body">
                    <div class="shortcut-group">
                        <h4>Navigation</h4>
                        <div class="shortcut-row"><kbd>j</kbd> <span>Next post</span></div>
                        <div class="shortcut-row"><kbd>k</kbd> <span>Previous post</span></div>
                        <div class="shortcut-row"><kbd>g</kbd> <kbd>h</kbd> <span>Go to home</span></div>
                        <div class="shortcut-row"><kbd>g</kbd> <kbd>f</kbd> <span>Go to forums</span></div>
                        <div class="shortcut-row"><kbd>g</kbd> <kbd>n</kbd> <span>Go to new posts</span></div>
                        <div class="shortcut-row"><kbd>g</kbd> <kbd>w</kbd> <span>Go to watched threads</span></div>
                    </div>
                    <div class="shortcut-group">
                        <h4>Actions</h4>
                        <div class="shortcut-row"><kbd>r</kbd> <span>Reply to thread</span></div>
                        <div class="shortcut-row"><kbd>q</kbd> <span>Quote focused post</span></div>
                        <div class="shortcut-row"><kbd>/</kbd> <span>Focus search</span></div>
                        <div class="shortcut-row"><kbd>?</kbd> <span>Show this help</span></div>
                        <div class="shortcut-row"><kbd>Esc</kbd> <span>Close modal / unfocus</span></div>
                    </div>
                </div>
                <div class="keyboard-help-footer">
                    <small>Press <kbd>?</kbd> or <kbd>Esc</kbd> to close</small>
                </div>
            </div>
        `;

        document.body.appendChild(modal);

        // Close handlers
        modal.querySelector('.keyboard-help-overlay').addEventListener('click', hideHelp);
        modal.querySelector('.keyboard-help-close').addEventListener('click', hideHelp);
    }

    /**
     * Hide keyboard shortcuts help modal
     */
    function hideHelp() {
        const modal = document.getElementById('keyboard-help-modal');
        if (modal) {
            modal.remove();
        }
    }

    /**
     * Close any open modals
     */
    function closeModals() {
        // Close keyboard help
        hideHelp();

        // Close report modal if open
        const reportModal = document.querySelector('.report-modal');
        if (reportModal) {
            reportModal.remove();
        }

        // Unfocus any focused element
        if (document.activeElement) {
            document.activeElement.blur();
        }

        // Remove post focus
        document.querySelectorAll('.post-focused').forEach(p => p.classList.remove('post-focused'));
        currentPostIndex = -1;
    }

    /**
     * Clear pending key sequence
     */
    function clearPending() {
        pendingPrefix = null;
        if (pendingTimeout) {
            clearTimeout(pendingTimeout);
            pendingTimeout = null;
        }
    }

    /**
     * Handle keydown events
     */
    function handleKeydown(e) {
        // Ignore if typing in input
        if (isTyping()) {
            // But still handle Escape
            if (e.key === 'Escape') {
                document.activeElement.blur();
                e.preventDefault();
            }
            return;
        }

        // Ignore if modifier keys are pressed (except Shift for ?)
        if (e.ctrlKey || e.altKey || e.metaKey) {
            return;
        }

        const key = e.key.toLowerCase();

        // Handle pending sequences (g + something)
        if (pendingPrefix === 'g') {
            clearPending();
            switch (key) {
                case 'h':
                    navigateTo('/');
                    e.preventDefault();
                    return;
                case 'f':
                    navigateTo('/forums');
                    e.preventDefault();
                    return;
                case 'n':
                    navigateTo('/recent/posts');
                    e.preventDefault();
                    return;
                case 'w':
                    navigateTo('/watched-threads');
                    e.preventDefault();
                    return;
            }
            // Unknown sequence, ignore
            return;
        }

        // Single key shortcuts
        switch (key) {
            case '?':
                showHelp();
                e.preventDefault();
                break;

            case 'escape':
                closeModals();
                e.preventDefault();
                break;

            case 'j':
                nextPost();
                e.preventDefault();
                break;

            case 'k':
                prevPost();
                e.preventDefault();
                break;

            case 'r':
                if (focusReply()) {
                    e.preventDefault();
                }
                break;

            case 'q':
                if (quoteCurrentPost()) {
                    e.preventDefault();
                }
                break;

            case '/':
                if (focusSearch()) {
                    e.preventDefault();
                }
                break;

            case 'g':
                // Start a sequence
                pendingPrefix = 'g';
                pendingTimeout = setTimeout(clearPending, SEQUENCE_TIMEOUT);
                e.preventDefault();
                break;
        }
    }

    /**
     * Initialize keyboard shortcuts
     */
    function init() {
        document.addEventListener('keydown', handleKeydown);

        // Reset post index when clicking elsewhere
        document.addEventListener('click', (e) => {
            if (!e.target.closest('.message')) {
                document.querySelectorAll('.post-focused').forEach(p => p.classList.remove('post-focused'));
                currentPostIndex = -1;
            }
        });
    }

    // Run on DOMContentLoaded
    if (document.readyState === 'loading') {
        document.addEventListener('DOMContentLoaded', init);
    } else {
        init();
    }

    // Expose for external use
    window.RuforoKeyboard = {
        showHelp,
        hideHelp
    };

})();
