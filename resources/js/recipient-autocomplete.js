/**
 * Recipient autocomplete for conversation recipients input
 * Shows a dropdown with matching usernames as user types
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

    // State for the input
    let dropdown = null;
    let selectedIndex = 0;
    let results = [];
    let currentInput = null;

    // Create dropdown element
    function createDropdown(input) {
        const existing = document.getElementById('recipient-dropdown');
        if (existing) existing.remove();

        const dd = document.createElement('div');
        dd.id = 'recipient-dropdown';
        dd.className = 'recipient-dropdown';
        dd.style.display = 'none';

        // Position dropdown relative to input's parent
        const parent = input.parentElement;
        if (parent.style.position !== 'relative' && parent.style.position !== 'absolute') {
            parent.style.position = 'relative';
        }
        parent.appendChild(dd);

        return dd;
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
            console.error('User search error:', e);
            return [];
        }
    }

    // Render dropdown with results
    function renderDropdown(input, searchResults) {
        results = searchResults;
        selectedIndex = 0;

        if (!dropdown) {
            dropdown = createDropdown(input);
        }

        if (results.length === 0) {
            dropdown.style.display = 'none';
            return;
        }

        dropdown.innerHTML = results.map((user, index) => `
            <div class="recipient-item${index === 0 ? ' recipient-item--selected' : ''}" data-index="${index}" data-username="${user.username}">
                <span class="recipient-username">${user.username}</span>
            </div>
        `).join('');

        // Position below input
        dropdown.style.left = '0';
        dropdown.style.top = (input.offsetHeight + 2) + 'px';
        dropdown.style.width = input.offsetWidth + 'px';
        dropdown.style.display = 'block';

        // Add click handlers
        dropdown.querySelectorAll('.recipient-item').forEach(item => {
            item.addEventListener('mousedown', (e) => {
                e.preventDefault();
                const username = item.dataset.username;
                insertUsername(input, username);
            });
        });
    }

    // Get the current word being typed (after last comma)
    function getCurrentWord(input) {
        const value = input.value;
        const cursorPos = input.selectionStart;
        const beforeCursor = value.substring(0, cursorPos);

        // Find last comma before cursor
        const lastComma = beforeCursor.lastIndexOf(',');
        const currentWord = beforeCursor.substring(lastComma + 1).trim();

        return {
            word: currentWord,
            startPos: lastComma + 1
        };
    }

    // Insert selected username
    function insertUsername(input, username) {
        const value = input.value;
        const cursorPos = input.selectionStart;
        const beforeCursor = value.substring(0, cursorPos);
        const afterCursor = value.substring(cursorPos);

        // Find last comma before cursor
        const lastComma = beforeCursor.lastIndexOf(',');
        const prefix = lastComma >= 0 ? beforeCursor.substring(0, lastComma + 1) + ' ' : '';

        // Build new value
        const newValue = prefix + username + (afterCursor.trim() ? ', ' + afterCursor.trim() : '');
        input.value = newValue;

        // Set cursor after inserted username
        const newPos = prefix.length + username.length;
        input.setSelectionRange(newPos, newPos);
        input.focus();

        hideDropdown();
    }

    // Hide dropdown
    function hideDropdown() {
        if (dropdown) {
            dropdown.style.display = 'none';
        }
        results = [];
        selectedIndex = 0;
    }

    // Update selected item
    function updateSelection(newIndex) {
        if (results.length === 0) return;

        selectedIndex = Math.max(0, Math.min(newIndex, results.length - 1));

        dropdown.querySelectorAll('.recipient-item').forEach((item, index) => {
            item.classList.toggle('recipient-item--selected', index === selectedIndex);
        });
    }

    // Debounced search
    const debouncedSearch = debounce(async (input, query) => {
        const searchResults = await searchUsernames(query);
        renderDropdown(input, searchResults);
    }, 150);

    // Handle input event
    function handleInput(e) {
        const input = e.target;
        currentInput = input;

        const { word } = getCurrentWord(input);

        if (word.length >= 1) {
            debouncedSearch(input, word);
        } else {
            hideDropdown();
        }
    }

    // Handle keydown event
    function handleKeydown(e) {
        if (!dropdown || dropdown.style.display === 'none') {
            return;
        }

        switch (e.key) {
            case 'ArrowDown':
                e.preventDefault();
                updateSelection(selectedIndex + 1);
                break;

            case 'ArrowUp':
                e.preventDefault();
                updateSelection(selectedIndex - 1);
                break;

            case 'Enter':
            case 'Tab':
                if (results.length > 0) {
                    e.preventDefault();
                    insertUsername(currentInput, results[selectedIndex].username);
                }
                break;

            case 'Escape':
                e.preventDefault();
                hideDropdown();
                break;
        }
    }

    // Handle blur event
    function handleBlur(e) {
        // Delay to allow click on dropdown
        setTimeout(() => {
            hideDropdown();
        }, 200);
    }

    // Initialize
    function init() {
        const input = document.getElementById('recipient_usernames');
        if (!input) return;

        input.addEventListener('input', handleInput);
        input.addEventListener('keydown', handleKeydown);
        input.addEventListener('blur', handleBlur);
    }

    // Run on DOMContentLoaded
    if (document.readyState === 'loading') {
        document.addEventListener('DOMContentLoaded', init);
    } else {
        init();
    }

})();
