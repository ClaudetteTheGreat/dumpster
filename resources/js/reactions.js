/**
 * Post Reactions System
 *
 * Handles loading, displaying, and toggling reactions on posts.
 */

// Cache for reaction types
let reactionTypesCache = null;

/**
 * Create and show reaction users overlay
 */
async function showReactionUsersOverlay(ugcId, reactionTypeId) {
    // Remove any existing overlay
    const existingOverlay = document.querySelector('.reaction-users-overlay');
    if (existingOverlay) {
        existingOverlay.remove();
    }

    // Create overlay
    const overlay = document.createElement('div');
    overlay.className = 'reaction-users-overlay';
    overlay.innerHTML = `
        <div class="reaction-users-modal">
            <div class="reaction-users-header">
                <span class="reaction-users-title">Loading...</span>
                <button type="button" class="reaction-users-close">&times;</button>
            </div>
            <div class="reaction-users-content">
                <div class="reaction-users-loading">Loading...</div>
            </div>
        </div>
    `;

    document.body.appendChild(overlay);

    // Close on overlay click or close button
    overlay.addEventListener('click', (e) => {
        if (e.target === overlay || e.target.classList.contains('reaction-users-close')) {
            overlay.remove();
        }
    });

    // Close on Escape key
    const escHandler = (e) => {
        if (e.key === 'Escape') {
            overlay.remove();
            document.removeEventListener('keydown', escHandler);
        }
    };
    document.addEventListener('keydown', escHandler);

    // Fetch users
    try {
        const response = await fetch(`/reactions/${ugcId}/users?reaction_type_id=${reactionTypeId}`);
        if (!response.ok) throw new Error('Failed to load users');
        const data = await response.json();

        // Update header with reaction info
        const icon = data.reaction_image_url
            ? `<img src="${data.reaction_image_url}" alt="${data.reaction_name}" class="reaction-users-icon" />`
            : `<span class="reaction-users-emoji">${data.reaction_emoji}</span>`;

        overlay.querySelector('.reaction-users-title').innerHTML = `${icon} ${data.reaction_name}`;

        // Build user list
        const contentEl = overlay.querySelector('.reaction-users-content');
        if (data.users.length === 0) {
            contentEl.innerHTML = '<div class="reaction-users-empty">No reactions</div>';
        } else {
            const usersHtml = data.users.map(user => {
                const avatar = user.avatar_url
                    ? `<img src="${user.avatar_url}" alt="${user.name}" class="reaction-user-avatar" />`
                    : `<div class="reaction-user-avatar reaction-user-avatar--placeholder">${user.name.charAt(0).toUpperCase()}</div>`;
                return `
                    <a href="/members/${user.id}" class="reaction-user-item">
                        ${avatar}
                        <span class="reaction-user-name">${user.name}</span>
                    </a>
                `;
            }).join('');
            contentEl.innerHTML = `<div class="reaction-users-list">${usersHtml}</div>`;
        }
    } catch (error) {
        console.error('Error loading reaction users:', error);
        overlay.querySelector('.reaction-users-content').innerHTML =
            '<div class="reaction-users-error">Failed to load users</div>';
    }
}

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
 * Render the reactions summary bar (XenForo-style)
 */
function renderReactionsSummary(ugcId, reactions, userReactions) {
    // Find the reactionsBar for this ugcId
    const reactionsBar = document.querySelector(`.reactionsBar[data-ugc-id="${ugcId}"]`);
    if (!reactionsBar) return;

    const summaryEl = reactionsBar.querySelector('.reactionsSummary');
    if (!summaryEl) return;

    // Sort reactions by count descending
    const sortedReactions = [...reactions].sort((a, b) => b.count - a.count);

    if (sortedReactions.length === 0) {
        summaryEl.innerHTML = '';
        return;
    }

    // Build HTML for reaction summary
    const html = sortedReactions.map(reaction => {
        const isUserReaction = userReactions.includes(reaction.reaction_type_id);
        const countClass = reaction.count > 1 ? 'react-multi' : 'react-solo';

        // Use image if available, otherwise emoji
        const icon = reaction.image_url
            ? `<img src="${reaction.image_url}" alt="${reaction.name}" title="${reaction.name}" class="reaction-sprite" />`
            : `<span class="reaction-emoji" title="${reaction.name}">${reaction.emoji}</span>`;

        return `<li class="${countClass}${isUserReaction ? ' react-user' : ''}">
                    <a class="reactionsBar-link" href="/reactions/${ugcId}?reaction_id=${reaction.reaction_type_id}" data-reaction-type="${reaction.reaction_type_id}">
                        <span class="reaction reaction--small">${icon}</span> ${reaction.count}
                    </a>
                </li>`;
    }).join('');

    summaryEl.innerHTML = html;

    // Add click handlers to view who reacted
    summaryEl.querySelectorAll('.reactionsBar-link').forEach(link => {
        link.addEventListener('click', async (e) => {
            e.preventDefault();
            const reactionTypeId = link.dataset.reactionType;
            showReactionUsersOverlay(ugcId, reactionTypeId);
        });
    });
}

/**
 * Render the reactions display for a container (legacy support)
 */
function renderReactionsDisplay(container, reactions, userReactions) {
    const ugcId = container.dataset.ugcId;

    // Also update the reactions summary bar
    renderReactionsSummary(ugcId, reactions, userReactions);

    // Legacy display element (if exists)
    const displayEl = container.querySelector('.reactions-display');
    if (!displayEl) return;

    // Sort reactions by count descending
    const sortedReactions = [...reactions].sort((a, b) => b.count - a.count);

    // Build HTML
    const html = sortedReactions.map(reaction => {
        const isUserReaction = userReactions.includes(reaction.reaction_type_id);
        const classes = ['reaction-badge'];
        if (isUserReaction) classes.push('reaction-badge--active');

        // Use image if available, otherwise emoji
        const icon = reaction.image_url
            ? `<img src="${reaction.image_url}" alt="${reaction.name}" class="reaction-image" />`
            : `<span class="reaction-emoji">${reaction.emoji}</span>`;

        return `<span class="${classes.join(' ')}"
                      data-reaction-type="${reaction.reaction_type_id}"
                      title="${reaction.name}">
                    ${icon}
                    <span class="reaction-count">${reaction.count}</span>
                </span>`;
    }).join('');

    displayEl.innerHTML = html;

    // Add click handlers to toggle reactions
    displayEl.querySelectorAll('.reaction-badge').forEach(badge => {
        badge.addEventListener('click', async () => {
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

        // Use image if available, otherwise emoji
        const icon = type.image_url
            ? `<img src="${type.image_url}" alt="${type.name}" class="reaction-picker-image" />`
            : type.emoji;

        return `<button type="button"
                        class="${classes.join(' ')}"
                        data-reaction-type="${type.id}"
                        title="${type.name}">
                    ${icon}
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
    // Find all reaction containers
    const containers = document.querySelectorAll('.reactions-container');

    // Only proceed if there are reaction containers on the page
    if (containers.length === 0) {
        return;
    }

    // Pre-load reaction types
    await loadReactionTypes();

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
