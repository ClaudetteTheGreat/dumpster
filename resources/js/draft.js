/**
 * Draft auto-save functionality
 * Automatically saves form content to localStorage to prevent data loss
 */

(function() {
    'use strict';

    const DRAFT_PREFIX = 'ruforo_draft_';
    const SAVE_DELAY = 2000; // 2 seconds after last keystroke
    const DRAFT_EXPIRY = 7 * 24 * 60 * 60 * 1000; // 7 days in milliseconds

    let saveTimeouts = {};

    /**
     * Generate a unique key for the draft based on form action
     */
    function getDraftKey(form) {
        const action = form.getAttribute('action') || '';

        // Thread reply: /threads/{id}/post-reply
        const threadMatch = action.match(/\/threads\/(\d+)\/post-reply/);
        if (threadMatch) {
            return DRAFT_PREFIX + 'thread_reply_' + threadMatch[1];
        }

        // New thread: /forums/{id}/post-thread
        const forumMatch = action.match(/\/forums\/(\d+)\/post-thread/);
        if (forumMatch) {
            return DRAFT_PREFIX + 'new_thread_' + forumMatch[1];
        }

        // Conversation reply or new conversation
        const convMatch = action.match(/\/conversations\/(\d+)\/reply/);
        if (convMatch) {
            return DRAFT_PREFIX + 'conv_reply_' + convMatch[1];
        }

        const newConvMatch = action.match(/\/conversations\/new/);
        if (newConvMatch) {
            return DRAFT_PREFIX + 'new_conversation';
        }

        return null;
    }

    /**
     * Get draft data from localStorage
     */
    function getDraft(key) {
        try {
            const data = localStorage.getItem(key);
            if (!data) return null;

            const draft = JSON.parse(data);

            // Check if draft has expired
            if (draft.timestamp && Date.now() - draft.timestamp > DRAFT_EXPIRY) {
                localStorage.removeItem(key);
                return null;
            }

            return draft;
        } catch (e) {
            console.error('Error reading draft:', e);
            return null;
        }
    }

    /**
     * Save draft data to localStorage
     */
    function saveDraft(key, data) {
        try {
            data.timestamp = Date.now();
            localStorage.setItem(key, JSON.stringify(data));
            return true;
        } catch (e) {
            console.error('Error saving draft:', e);
            return false;
        }
    }

    /**
     * Clear draft from localStorage
     */
    function clearDraft(key) {
        try {
            localStorage.removeItem(key);
        } catch (e) {
            console.error('Error clearing draft:', e);
        }
    }

    /**
     * Show draft saved indicator
     */
    function showSavedIndicator(form) {
        let indicator = form.querySelector('.draft-indicator');

        if (!indicator) {
            indicator = document.createElement('span');
            indicator.className = 'draft-indicator';

            // Find a good place to insert the indicator
            const submitBtn = form.querySelector('button[type="submit"], button:not([type])');
            if (submitBtn) {
                submitBtn.parentNode.insertBefore(indicator, submitBtn.nextSibling);
            } else {
                form.appendChild(indicator);
            }
        }

        indicator.textContent = 'Draft saved';
        indicator.classList.add('draft-indicator--visible');

        // Hide after 2 seconds
        setTimeout(() => {
            indicator.classList.remove('draft-indicator--visible');
        }, 2000);
    }

    /**
     * Show draft restored indicator
     */
    function showRestoredIndicator(form) {
        let indicator = form.querySelector('.draft-indicator');

        if (!indicator) {
            indicator = document.createElement('span');
            indicator.className = 'draft-indicator';

            const submitBtn = form.querySelector('button[type="submit"], button:not([type])');
            if (submitBtn) {
                submitBtn.parentNode.insertBefore(indicator, submitBtn.nextSibling);
            } else {
                form.appendChild(indicator);
            }
        }

        indicator.textContent = 'Draft restored';
        indicator.classList.add('draft-indicator--visible', 'draft-indicator--restored');

        // Add clear button
        const clearBtn = document.createElement('button');
        clearBtn.type = 'button';
        clearBtn.className = 'draft-clear-btn';
        clearBtn.textContent = 'Clear draft';
        clearBtn.onclick = function() {
            const key = getDraftKey(form);
            if (key) {
                clearDraft(key);
                // Clear form fields
                const textarea = form.querySelector('textarea[name="content"]');
                if (textarea) textarea.value = '';
                const titleInput = form.querySelector('input[name="title"]');
                if (titleInput) titleInput.value = '';
                const subtitleInput = form.querySelector('input[name="subtitle"]');
                if (subtitleInput) subtitleInput.value = '';
            }
            indicator.classList.remove('draft-indicator--visible', 'draft-indicator--restored');
            clearBtn.remove();
        };
        indicator.appendChild(document.createTextNode(' '));
        indicator.appendChild(clearBtn);

        // Don't auto-hide when showing restored
    }

    /**
     * Collect form data for saving
     */
    function collectFormData(form) {
        const data = {};

        // Get textarea content (supports WYSIWYG editor)
        const textarea = form.querySelector('textarea[name="content"]');
        if (textarea) {
            // Check for WYSIWYG editor and get content from it
            const container = textarea.closest('.bbcode-editor-container');
            if (container && typeof window.getWysiwygEditor === 'function') {
                const editor = window.getWysiwygEditor(container);
                if (editor) {
                    data.content = editor.getContent();
                } else {
                    data.content = textarea.value;
                }
            } else {
                data.content = textarea.value;
            }
        }

        // Get title (for new threads)
        const titleInput = form.querySelector('input[name="title"]');
        if (titleInput) {
            data.title = titleInput.value;
        }

        // Get subtitle (for new threads)
        const subtitleInput = form.querySelector('input[name="subtitle"]');
        if (subtitleInput) {
            data.subtitle = subtitleInput.value;
        }

        return data;
    }

    /**
     * Check if form has meaningful content to save
     */
    function hasContent(data) {
        return (data.content && data.content.trim().length > 0) ||
               (data.title && data.title.trim().length > 0);
    }

    /**
     * Restore form data from draft
     */
    function restoreFormData(form, data) {
        if (data.content) {
            const textarea = form.querySelector('textarea[name="content"]');
            if (textarea && !textarea.value.trim()) {
                textarea.value = data.content;
                // Trigger input event for character counter
                textarea.dispatchEvent(new Event('input', { bubbles: true }));
            }
        }

        if (data.title) {
            const titleInput = form.querySelector('input[name="title"]');
            if (titleInput && !titleInput.value.trim()) {
                titleInput.value = data.title;
            }
        }

        if (data.subtitle) {
            const subtitleInput = form.querySelector('input[name="subtitle"]');
            if (subtitleInput && !subtitleInput.value.trim()) {
                subtitleInput.value = data.subtitle;
            }
        }
    }

    /**
     * Schedule a draft save with debouncing
     */
    function scheduleSave(form, key) {
        // Clear any pending save
        if (saveTimeouts[key]) {
            clearTimeout(saveTimeouts[key]);
        }

        // Schedule new save
        saveTimeouts[key] = setTimeout(() => {
            const data = collectFormData(form);

            if (hasContent(data)) {
                if (saveDraft(key, data)) {
                    showSavedIndicator(form);
                }
            } else {
                // Clear draft if content is empty
                clearDraft(key);
            }
        }, SAVE_DELAY);
    }

    /**
     * Initialize draft functionality for a form
     */
    function initForm(form) {
        const key = getDraftKey(form);
        if (!key) return;

        // Restore existing draft
        const draft = getDraft(key);
        if (draft && hasContent(draft)) {
            restoreFormData(form, draft);
            showRestoredIndicator(form);
        }

        // Listen for input changes
        const textarea = form.querySelector('textarea[name="content"]');
        const titleInput = form.querySelector('input[name="title"]');
        const subtitleInput = form.querySelector('input[name="subtitle"]');

        const handleInput = () => scheduleSave(form, key);

        if (textarea) {
            textarea.addEventListener('input', handleInput);
        }
        if (titleInput) {
            titleInput.addEventListener('input', handleInput);
        }
        if (subtitleInput) {
            subtitleInput.addEventListener('input', handleInput);
        }

        // Clear draft on successful submit
        form.addEventListener('submit', () => {
            // Clear any pending save
            if (saveTimeouts[key]) {
                clearTimeout(saveTimeouts[key]);
            }
            // Clear the draft
            clearDraft(key);
        });
    }

    /**
     * Initialize all forms on the page
     */
    function init() {
        // Find all forms that could have drafts
        const forms = document.querySelectorAll('form[action*="/post-reply"], form[action*="/post-thread"], form[action*="/conversations"]');

        forms.forEach(form => {
            initForm(form);
        });
    }

    // Run on DOMContentLoaded
    if (document.readyState === 'loading') {
        document.addEventListener('DOMContentLoaded', init);
    } else {
        init();
    }

    // Expose for external use
    window.RuforoDraft = {
        getDraft,
        clearDraft,
        getDraftKey
    };

})();
