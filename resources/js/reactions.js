/**
 * Post Reactions System
 *
 * Handles loading, displaying, and toggling reactions on posts.
 */

// Cache for reaction types
let reactionTypesCache = null;

/**
 * Load all available reaction types
 */
async function loadReactionTypes() {
    if (reactionTypesCache) return reactionTypesCache;

    try {
        const response = await fetch('/reactions/types');
        if (!response.ok) throw new Error('Failed to load reaction types');
        reactionTypesCache = await response.json();
        return reactionTypesCache;
    } catch (error) {
        console.error('Error loading reaction types:', error);
        return [];
    }
}

/**
 * Load reactions for a specific UGC item
 */
async function loadReactions(ugcId) {
    try {
        const response = await fetch(`/reactions/${ugcId}`);
        if (!response.ok) throw new Error('Failed to load reactions');
        return await response.json();
    } catch (error) {
        console.error('Error loading reactions:', error);
        return { reactions: [], user_reactions: [] };
    }
}

/**
 * Toggle a reaction on a UGC item
 */
async function toggleReaction(ugcId, reactionTypeId, csrfToken) {
    try {
        const response = await fetch(`/reactions/${ugcId}/${reactionTypeId}`, {
            method: 'POST',
            headers: {
                'Content-Type': 'application/x-www-form-urlencoded',
            },
            body: `csrf_token=${encodeURIComponent(csrfToken)}`,
        });

        if (!response.ok) {
            if (response.status === 401) {
                alert('You must be logged in to react');
                return null;
            }
            throw new Error('Failed to toggle reaction');
        }

        return await response.json();
    } catch (error) {
        console.error('Error toggling reaction:', error);
        return null;
    }
}

/**
 * Render the reactions display for a container
 */
function renderReactionsDisplay(container, reactions, userReactions) {
    const displayEl = container.querySelector('.reactions-display');
    if (!displayEl) return;

    // Sort reactions by count descending
    const sortedReactions = [...reactions].sort((a, b) => b.count - a.count);

    // Build HTML
    const html = sortedReactions.map(reaction => {
        const isUserReaction = userReactions.includes(reaction.reaction_type_id);
        const classes = ['reaction-badge'];
        if (isUserReaction) classes.push('reaction-badge--active');

        return `<span class="${classes.join(' ')}"
                      data-reaction-type="${reaction.reaction_type_id}"
                      title="${reaction.name}">
                    <span class="reaction-emoji">${reaction.emoji}</span>
                    <span class="reaction-count">${reaction.count}</span>
                </span>`;
    }).join('');

    displayEl.innerHTML = html;

    // Add click handlers to toggle reactions
    displayEl.querySelectorAll('.reaction-badge').forEach(badge => {
        badge.addEventListener('click', async () => {
            const ugcId = container.dataset.ugcId;
            const csrfToken = container.dataset.csrf;
            const reactionTypeId = badge.dataset.reactionType;

            const result = await toggleReaction(ugcId, reactionTypeId, csrfToken);
            if (result && result.success) {
                // Reload and re-render reactions
                const data = await loadReactions(ugcId);
                renderReactionsDisplay(container, data.reactions, data.user_reactions);
            }
        });
    });
}

/**
 * Render the reaction picker dropdown
 */
async function renderReactionPicker(container) {
    const dropdown = container.querySelector('.reaction-picker-dropdown');
    if (!dropdown) return;

    const reactionTypes = await loadReactionTypes();
    const ugcId = container.dataset.ugcId;
    const csrfToken = container.dataset.csrf;

    // Get current user reactions
    const data = await loadReactions(ugcId);
    const userReactions = data.user_reactions;

    const html = reactionTypes.map(type => {
        const isActive = userReactions.includes(type.id);
        const classes = ['reaction-option'];
        if (isActive) classes.push('reaction-option--active');

        return `<button type="button"
                        class="${classes.join(' ')}"
                        data-reaction-type="${type.id}"
                        title="${type.name}">
                    ${type.emoji}
                </button>`;
    }).join('');

    dropdown.innerHTML = html;

    // Add click handlers
    dropdown.querySelectorAll('.reaction-option').forEach(button => {
        button.addEventListener('click', async (e) => {
            e.stopPropagation();
            const reactionTypeId = button.dataset.reactionType;

            const result = await toggleReaction(ugcId, reactionTypeId, csrfToken);
            if (result && result.success) {
                // Update button state
                button.classList.toggle('reaction-option--active', result.added);

                // Reload and re-render reactions display
                const data = await loadReactions(ugcId);
                renderReactionsDisplay(container, data.reactions, data.user_reactions);

                // Hide dropdown
                dropdown.style.display = 'none';
            }
        });
    });
}

/**
 * Initialize reactions on page load
 */
document.addEventListener('DOMContentLoaded', async () => {
    // Pre-load reaction types
    await loadReactionTypes();

    // Find all reaction containers
    const containers = document.querySelectorAll('.reactions-container');

    // Load and display reactions for each container
    for (const container of containers) {
        const ugcId = container.dataset.ugcId;
        if (!ugcId) continue;

        // Load reactions
        const data = await loadReactions(ugcId);
        renderReactionsDisplay(container, data.reactions, data.user_reactions);

        // Set up picker toggle
        const toggle = container.querySelector('.reaction-picker-toggle');
        const dropdown = container.querySelector('.reaction-picker-dropdown');

        if (toggle && dropdown) {
            let pickerRendered = false;

            toggle.addEventListener('click', async (e) => {
                e.stopPropagation();

                // Render picker on first open
                if (!pickerRendered) {
                    await renderReactionPicker(container);
                    pickerRendered = true;
                }

                // Toggle dropdown visibility
                const isVisible = dropdown.style.display !== 'none';
                dropdown.style.display = isVisible ? 'none' : 'flex';

                // Close other dropdowns
                document.querySelectorAll('.reaction-picker-dropdown').forEach(d => {
                    if (d !== dropdown) d.style.display = 'none';
                });
            });
        }
    }

    // Close dropdowns when clicking outside
    document.addEventListener('click', () => {
        document.querySelectorAll('.reaction-picker-dropdown').forEach(d => {
            d.style.display = 'none';
        });
    });
});
