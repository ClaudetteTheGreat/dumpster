/**
 * Quote reply functionality for posts
 * Allows users to click a Quote button to insert quoted content into the reply textarea
 */

(function() {
    'use strict';

    // Find the reply textarea on the page
    function getReplyTextarea() {
        return document.querySelector('form[action*="post-reply"] textarea[name="content"]');
    }

    // Decode HTML entities in content
    function decodeHtmlEntities(text) {
        const textarea = document.createElement('textarea');
        textarea.innerHTML = text;
        return textarea.value;
    }

    // Insert quote into textarea
    function insertQuote(username, content) {
        const textarea = getReplyTextarea();
        if (!textarea) {
            alert('Reply form not found. You may need to scroll down to the reply form.');
            return;
        }

        // Decode any HTML entities in the content
        const decodedContent = decodeHtmlEntities(content);

        // Build the quote BBCode
        const quote = `[quote=${username}]${decodedContent}[/quote]\n\n`;

        // Get current cursor position
        const start = textarea.selectionStart;
        const end = textarea.selectionEnd;
        const currentValue = textarea.value;

        // Insert quote at cursor position (or append if no selection)
        if (start === end && start === 0 && currentValue.length === 0) {
            // Empty textarea, just set the value
            textarea.value = quote;
        } else {
            // Insert at cursor position
            textarea.value = currentValue.substring(0, start) + quote + currentValue.substring(end);
        }

        // Move cursor to end of inserted quote
        const newPosition = start + quote.length;
        textarea.setSelectionRange(newPosition, newPosition);

        // Focus the textarea
        textarea.focus();

        // Scroll textarea into view
        textarea.scrollIntoView({ behavior: 'smooth', block: 'center' });

        // Trigger input event for character counter
        textarea.dispatchEvent(new Event('input', { bubbles: true }));
    }

    // Handle quote button click
    function handleQuoteClick(e) {
        const button = e.target.closest('.quote-btn');
        if (!button) return;

        const username = button.dataset.username || 'Unknown';
        const content = button.dataset.content || '';

        if (!content.trim()) {
            alert('This post has no content to quote.');
            return;
        }

        insertQuote(username, content);
    }

    // Initialize quote functionality
    function init() {
        // Use event delegation on document for quote buttons
        document.addEventListener('click', handleQuoteClick);
    }

    // Run on DOMContentLoaded
    if (document.readyState === 'loading') {
        document.addEventListener('DOMContentLoaded', init);
    } else {
        init();
    }

})();
