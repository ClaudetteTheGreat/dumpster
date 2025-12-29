/**
 * Quote reply functionality for posts
 * Supports both single-quote (immediate insert) and multi-quote (queue and insert)
 */

(function() {
    'use strict';

    const STORAGE_KEY = 'ruforo_multi_quotes';

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

    // Get stored quotes from localStorage
    function getStoredQuotes() {
        try {
            const stored = localStorage.getItem(STORAGE_KEY);
            return stored ? JSON.parse(stored) : [];
        } catch (e) {
            console.error('Error reading stored quotes:', e);
            return [];
        }
    }

    // Save quotes to localStorage
    function saveQuotes(quotes) {
        try {
            localStorage.setItem(STORAGE_KEY, JSON.stringify(quotes));
        } catch (e) {
            console.error('Error saving quotes:', e);
        }
    }

    // Clear all stored quotes
    function clearQuotes() {
        localStorage.removeItem(STORAGE_KEY);
        updateMultiQuoteUI();
        updateAllAddQuoteButtons();
    }

    // Add a quote to the queue
    function addQuoteToQueue(postId, username, content, threadId) {
        const quotes = getStoredQuotes();

        // Check if already in queue
        if (quotes.some(q => q.postId === postId)) {
            return false; // Already queued
        }

        quotes.push({
            postId: postId,
            username: username,
            content: content,
            threadId: threadId,
            addedAt: Date.now()
        });

        saveQuotes(quotes);
        updateMultiQuoteUI();
        return true;
    }

    // Remove a quote from the queue
    function removeQuoteFromQueue(postId) {
        const quotes = getStoredQuotes();
        const filtered = quotes.filter(q => q.postId !== postId);
        saveQuotes(filtered);
        updateMultiQuoteUI();
    }

    // Check if a post is in the queue
    function isPostInQueue(postId) {
        const quotes = getStoredQuotes();
        return quotes.some(q => q.postId === postId);
    }

    // Build quote BBCode from a quote object
    // Format: [quote=username;thread_id;post_id] for linked quotes
    function buildQuoteBBCode(quote) {
        const decodedContent = decodeHtmlEntities(quote.content);
        if (quote.threadId && quote.postId) {
            return `[quote=${quote.username};${quote.threadId};${quote.postId}]${decodedContent}[/quote]`;
        }
        return `[quote=${quote.username}]${decodedContent}[/quote]`;
    }

    // Insert a single quote into textarea (supports WYSIWYG editor)
    async function insertQuote(username, content, threadId, postId) {
        const textarea = getReplyTextarea();
        if (!textarea) {
            alert('Reply form not found. You may need to scroll down to the reply form.');
            return;
        }

        const decodedContent = decodeHtmlEntities(content);
        let quote;
        if (threadId && postId) {
            quote = `[quote=${username};${threadId};${postId}]${decodedContent}[/quote]\n\n`;
        } else {
            quote = `[quote=${username}]${decodedContent}[/quote]\n\n`;
        }

        // Use WYSIWYG-aware insert if available
        if (typeof window.insertEditorContent === 'function') {
            await window.insertEditorContent(textarea.id || 'content', quote);
            textarea.scrollIntoView({ behavior: 'smooth', block: 'center' });
            return;
        }

        // Fallback: direct textarea insert
        const start = textarea.selectionStart;
        const end = textarea.selectionEnd;
        const currentValue = textarea.value;

        if (start === end && start === 0 && currentValue.length === 0) {
            textarea.value = quote;
        } else {
            textarea.value = currentValue.substring(0, start) + quote + currentValue.substring(end);
        }

        const newPosition = start + quote.length;
        textarea.setSelectionRange(newPosition, newPosition);
        textarea.focus();
        textarea.scrollIntoView({ behavior: 'smooth', block: 'center' });
        textarea.dispatchEvent(new Event('input', { bubbles: true }));
    }

    // Insert all queued quotes into textarea (supports WYSIWYG editor)
    async function insertAllQuotes() {
        const textarea = getReplyTextarea();
        if (!textarea) {
            alert('Reply form not found. You may need to scroll down to the reply form.');
            return;
        }

        const quotes = getStoredQuotes();
        if (quotes.length === 0) {
            alert('No quotes selected.');
            return;
        }

        // Sort by addedAt to maintain order
        quotes.sort((a, b) => a.addedAt - b.addedAt);

        // Build all quotes
        const allQuotes = quotes.map(q => buildQuoteBBCode(q)).join('\n\n') + '\n\n';

        // Use WYSIWYG-aware insert if available
        if (typeof window.insertEditorContent === 'function') {
            await window.insertEditorContent(textarea.id || 'content', allQuotes);
            textarea.scrollIntoView({ behavior: 'smooth', block: 'center' });
            clearQuotes();
            return;
        }

        // Fallback: direct textarea insert
        const start = textarea.selectionStart;
        const currentValue = textarea.value;

        if (currentValue.length === 0) {
            textarea.value = allQuotes;
        } else {
            textarea.value = currentValue.substring(0, start) + allQuotes + currentValue.substring(start);
        }

        const newPosition = start + allQuotes.length;
        textarea.setSelectionRange(newPosition, newPosition);
        textarea.focus();
        textarea.scrollIntoView({ behavior: 'smooth', block: 'center' });
        textarea.dispatchEvent(new Event('input', { bubbles: true }));

        // Clear the queue after inserting
        clearQuotes();
    }

    // Update the floating multi-quote indicator
    function updateMultiQuoteUI() {
        const quotes = getStoredQuotes();
        let indicator = document.getElementById('multi-quote-indicator');

        if (quotes.length === 0) {
            if (indicator) {
                indicator.remove();
            }
            return;
        }

        if (!indicator) {
            indicator = document.createElement('div');
            indicator.id = 'multi-quote-indicator';
            indicator.innerHTML = `
                <span class="mq-count"></span>
                <button type="button" class="mq-insert" title="Insert all quotes">Insert Quotes</button>
                <button type="button" class="mq-clear" title="Clear all quotes">Clear</button>
            `;
            document.body.appendChild(indicator);

            // Add event listeners
            indicator.querySelector('.mq-insert').addEventListener('click', insertAllQuotes);
            indicator.querySelector('.mq-clear').addEventListener('click', clearQuotes);
        }

        indicator.querySelector('.mq-count').textContent = `${quotes.length} quote${quotes.length !== 1 ? 's' : ''} selected`;
    }

    // Update all +Quote buttons to show correct state
    function updateAllAddQuoteButtons() {
        document.querySelectorAll('.add-quote-btn').forEach(btn => {
            const postId = btn.dataset.postId;
            if (isPostInQueue(postId)) {
                btn.textContent = '-Quote';
                btn.classList.add('quote-selected');
                btn.title = 'Remove from quotes';
            } else {
                btn.textContent = '+Quote';
                btn.classList.remove('quote-selected');
                btn.title = 'Add to quotes';
            }
        });
    }

    // Handle single quote button click (immediate insert)
    function handleQuoteClick(e) {
        const button = e.target.closest('.quote-btn');
        if (!button) return;

        const username = button.dataset.username || 'Unknown';
        const content = button.dataset.content || '';
        const threadId = button.dataset.threadId || '';
        const postId = button.dataset.postId || '';

        if (!content.trim()) {
            alert('This post has no content to quote.');
            return;
        }

        insertQuote(username, content, threadId, postId);
    }

    // Handle add/remove quote button click (multi-quote)
    function handleAddQuoteClick(e) {
        const button = e.target.closest('.add-quote-btn');
        if (!button) return;

        const postId = button.dataset.postId;
        const username = button.dataset.username || 'Unknown';
        const content = button.dataset.content || '';
        const threadId = button.dataset.threadId || '';

        if (!content.trim()) {
            alert('This post has no content to quote.');
            return;
        }

        if (isPostInQueue(postId)) {
            removeQuoteFromQueue(postId);
            button.textContent = '+Quote';
            button.classList.remove('quote-selected');
            button.title = 'Add to quotes';
        } else {
            addQuoteToQueue(postId, username, content, threadId);
            button.textContent = '-Quote';
            button.classList.add('quote-selected');
            button.title = 'Remove from quotes';
        }
    }

    // Handle quote link clicks - scroll if on same page
    function handleQuoteLinkClick(e) {
        const link = e.target.closest('.quote-link');
        if (!link) return;

        const href = link.getAttribute('href');
        if (!href) return;

        // Extract post ID from href like /threads/123/post-456
        const match = href.match(/\/threads\/(\d+)\/post-(\d+)/);
        if (!match) return;

        const postId = match[2];

        // Check if the target post is on this page
        const targetPost = document.querySelector(`a[href="/threads/${match[1]}/post-${postId}"]`);
        if (targetPost) {
            // Find the parent message element
            const messageEl = targetPost.closest('.message');
            if (messageEl) {
                e.preventDefault();
                messageEl.scrollIntoView({ behavior: 'smooth', block: 'start' });

                // Briefly highlight the post
                messageEl.classList.add('post-focused');
                setTimeout(() => {
                    messageEl.classList.remove('post-focused');
                }, 2000);
            }
        }
        // If not found, let the normal navigation happen
    }

    // Initialize
    function init() {
        // Event delegation for quote buttons
        document.addEventListener('click', handleQuoteClick);
        document.addEventListener('click', handleAddQuoteClick);
        document.addEventListener('click', handleQuoteLinkClick);

        // Update UI on page load
        updateMultiQuoteUI();
        updateAllAddQuoteButtons();
    }

    // Run on DOMContentLoaded
    if (document.readyState === 'loading') {
        document.addEventListener('DOMContentLoaded', init);
    } else {
        init();
    }

    // Expose for external use if needed
    window.RuforoMultiQuote = {
        getStoredQuotes,
        clearQuotes,
        insertAllQuotes
    };

})();
