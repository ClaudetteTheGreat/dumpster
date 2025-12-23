/**
 * @mention autocomplete for post editor textareas
 * Detects when user types @ and shows a dropdown with matching usernames
 */

(function() {
    'use strict';

    // Debounce helper to limit API calls
    function debounce(func, wait) {
        let timeout;
        return function executedFunction(...args) {
            const later = () => {
                clearTimeout(timeout);
                func(...args);
            };
            clearTimeout(timeout);
            timeout = setTimeout(later, wait);
        };
    }

    // State for each textarea
    const textareaStates = new WeakMap();

    function getState(textarea) {
        if (!textareaStates.has(textarea)) {
            textareaStates.set(textarea, {
                dropdown: null,
                mentionStart: -1,
                selectedIndex: 0,
                results: []
            });
        }
        return textareaStates.get(textarea);
    }

    // Create dropdown element
    function createDropdown(textarea) {
        const dropdown = document.createElement('div');
        dropdown.className = 'mention-dropdown';
        dropdown.style.display = 'none';

        // Position dropdown relative to textarea's parent
        const parent = textarea.parentElement;
        if (parent.style.position !== 'relative' && parent.style.position !== 'absolute') {
            parent.style.position = 'relative';
        }
        parent.appendChild(dropdown);

        return dropdown;
    }

    // Get caret coordinates in textarea
    function getCaretCoordinates(textarea) {
        const div = document.createElement('div');
        const style = getComputedStyle(textarea);

        // Copy styles that affect text layout
        const properties = [
            'fontFamily', 'fontSize', 'fontWeight', 'fontStyle',
            'letterSpacing', 'textTransform', 'wordSpacing', 'textIndent',
            'whiteSpace', 'wordWrap', 'lineHeight', 'padding', 'border', 'boxSizing'
        ];
        properties.forEach(prop => {
            div.style[prop] = style[prop];
        });

        div.style.position = 'absolute';
        div.style.visibility = 'hidden';
        div.style.whiteSpace = 'pre-wrap';
        div.style.width = textarea.offsetWidth + 'px';

        // Get text up to caret
        const text = textarea.value.substring(0, textarea.selectionStart);
        div.textContent = text;

        // Add a span to mark the caret position
        const span = document.createElement('span');
        span.textContent = '|';
        div.appendChild(span);

        document.body.appendChild(div);

        const coordinates = {
            top: span.offsetTop - textarea.scrollTop,
            left: span.offsetLeft
        };

        document.body.removeChild(div);

        return coordinates;
    }

    // Position dropdown near caret
    function positionDropdown(textarea, dropdown) {
        const coords = getCaretCoordinates(textarea);
        const textareaRect = textarea.getBoundingClientRect();
        const parentRect = textarea.parentElement.getBoundingClientRect();

        // Position relative to parent
        dropdown.style.left = (textarea.offsetLeft + coords.left) + 'px';
        dropdown.style.top = (textarea.offsetTop + coords.top + 24) + 'px';

        // Ensure dropdown doesn't go off-screen
        const dropdownRect = dropdown.getBoundingClientRect();
        if (dropdownRect.right > window.innerWidth) {
            dropdown.style.left = (window.innerWidth - dropdownRect.width - 10) + 'px';
        }
    }

    // Search for usernames
    async function searchUsernames(query) {
        if (!query || query.length < 1) {
            return [];
        }

        try {
            const response = await fetch(`/api/users/search?q=${encodeURIComponent(query)}`);
            if (!response.ok) {
                return [];
            }
            return await response.json();
        } catch (e) {
            console.error('Mention search error:', e);
            return [];
        }
    }

    // Render dropdown with results
    function renderDropdown(textarea, results) {
        const state = getState(textarea);
        state.results = results;
        state.selectedIndex = 0;

        if (!state.dropdown) {
            state.dropdown = createDropdown(textarea);
        }

        if (results.length === 0) {
            state.dropdown.style.display = 'none';
            return;
        }

        state.dropdown.innerHTML = results.map((user, index) => `
            <div class="mention-item${index === 0 ? ' mention-item--selected' : ''}" data-index="${index}" data-username="${user.username}">
                <span class="mention-username">@${user.username}</span>
            </div>
        `).join('');

        positionDropdown(textarea, state.dropdown);
        state.dropdown.style.display = 'block';

        // Add click handlers
        state.dropdown.querySelectorAll('.mention-item').forEach(item => {
            item.addEventListener('mousedown', (e) => {
                e.preventDefault();
                const username = item.dataset.username;
                insertMention(textarea, username);
            });
        });
    }

    // Insert selected mention into textarea
    function insertMention(textarea, username) {
        const state = getState(textarea);
        const before = textarea.value.substring(0, state.mentionStart);
        const after = textarea.value.substring(textarea.selectionStart);

        textarea.value = before + '@' + username + ' ' + after;

        // Set cursor position after the inserted mention
        const newPosition = state.mentionStart + username.length + 2; // +2 for @ and space
        textarea.setSelectionRange(newPosition, newPosition);
        textarea.focus();

        hideDropdown(textarea);

        // Trigger input event for char counter
        textarea.dispatchEvent(new Event('input', { bubbles: true }));
    }

    // Hide dropdown
    function hideDropdown(textarea) {
        const state = getState(textarea);
        if (state.dropdown) {
            state.dropdown.style.display = 'none';
        }
        state.mentionStart = -1;
        state.results = [];
    }

    // Update selected item
    function updateSelection(textarea, newIndex) {
        const state = getState(textarea);
        if (state.results.length === 0) return;

        state.selectedIndex = Math.max(0, Math.min(newIndex, state.results.length - 1));

        state.dropdown.querySelectorAll('.mention-item').forEach((item, index) => {
            item.classList.toggle('mention-item--selected', index === state.selectedIndex);
        });
    }

    // Extract mention query from text
    function extractMentionQuery(text, cursorPos) {
        // Look backwards from cursor for @
        let start = cursorPos - 1;
        while (start >= 0) {
            const char = text[start];
            if (char === '@') {
                // Check if @ is at start or preceded by whitespace
                if (start === 0 || /\s/.test(text[start - 1])) {
                    const query = text.substring(start + 1, cursorPos);
                    // Only valid if query contains valid username chars
                    if (/^[a-zA-Z0-9_-]*$/.test(query)) {
                        return { start, query };
                    }
                }
                return null;
            }
            if (/\s/.test(char)) {
                return null;
            }
            start--;
        }
        return null;
    }

    // Debounced search
    const debouncedSearch = debounce(async (textarea, query) => {
        const results = await searchUsernames(query);
        renderDropdown(textarea, results);
    }, 150);

    // Handle input event
    function handleInput(e) {
        const textarea = e.target;
        const text = textarea.value;
        const cursorPos = textarea.selectionStart;
        const state = getState(textarea);

        const mention = extractMentionQuery(text, cursorPos);

        if (mention) {
            state.mentionStart = mention.start;
            if (mention.query.length >= 1) {
                debouncedSearch(textarea, mention.query);
            } else {
                hideDropdown(textarea);
            }
        } else {
            hideDropdown(textarea);
        }
    }

    // Handle keydown event
    function handleKeydown(e) {
        const textarea = e.target;
        const state = getState(textarea);

        if (!state.dropdown || state.dropdown.style.display === 'none') {
            return;
        }

        switch (e.key) {
            case 'ArrowDown':
                e.preventDefault();
                updateSelection(textarea, state.selectedIndex + 1);
                break;

            case 'ArrowUp':
                e.preventDefault();
                updateSelection(textarea, state.selectedIndex - 1);
                break;

            case 'Enter':
            case 'Tab':
                if (state.results.length > 0) {
                    e.preventDefault();
                    insertMention(textarea, state.results[state.selectedIndex].username);
                }
                break;

            case 'Escape':
                e.preventDefault();
                hideDropdown(textarea);
                break;
        }
    }

    // Handle blur event
    function handleBlur(e) {
        // Delay to allow click on dropdown
        setTimeout(() => {
            hideDropdown(e.target);
        }, 200);
    }

    // Initialize mention autocomplete on a textarea
    function initMentionAutocomplete(textarea) {
        if (textarea.dataset.mentionInit) return;
        textarea.dataset.mentionInit = 'true';

        textarea.addEventListener('input', handleInput);
        textarea.addEventListener('keydown', handleKeydown);
        textarea.addEventListener('blur', handleBlur);
    }

    // Initialize on all content textareas
    function init() {
        document.querySelectorAll('textarea[name="content"]').forEach(initMentionAutocomplete);
    }

    // Run on DOMContentLoaded
    if (document.readyState === 'loading') {
        document.addEventListener('DOMContentLoaded', init);
    } else {
        init();
    }

    // Also observe for dynamically added textareas
    const observer = new MutationObserver((mutations) => {
        mutations.forEach((mutation) => {
            mutation.addedNodes.forEach((node) => {
                if (node.nodeType === Node.ELEMENT_NODE) {
                    if (node.matches && node.matches('textarea[name="content"]')) {
                        initMentionAutocomplete(node);
                    }
                    node.querySelectorAll && node.querySelectorAll('textarea[name="content"]').forEach(initMentionAutocomplete);
                }
            });
        });
    });
    observer.observe(document.body, { childList: true, subtree: true });

})();
