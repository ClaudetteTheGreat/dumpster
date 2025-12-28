/**
 * URL Unfurling - fetches and displays rich previews for URLs
 * Looks for .unfurl-container elements and hydrates them with metadata
 */

/**
 * Fetch unfurl data for a URL
 * @param {string} url - The URL to unfurl
 * @returns {Promise<Object>} - The unfurl metadata
 */
async function fetchUnfurl(url) {
    const response = await fetch(`/api/unfurl?url=${encodeURIComponent(url)}`);
    if (!response.ok) {
        throw new Error(`Failed to fetch unfurl data: ${response.status}`);
    }
    return response.json();
}

/**
 * Render unfurl preview card - dispatches to site-specific renderers
 * @param {Object} data - The unfurl metadata
 * @returns {string} - HTML string for the preview card
 */
function renderUnfurlCard(data) {
    if (!data.success && !data.site_type) {
        return ''; // Don't show anything if unfurl failed and no site type
    }

    // Dispatch to site-specific renderers
    switch (data.site_type) {
        case 'youtube':
            return renderYouTubeCard(data);
        case 'twitter':
            return renderTwitterCard(data);
        case 'github':
            return renderGitHubCard(data);
        default:
            return renderGenericCard(data);
    }
}

/**
 * Render YouTube video card with click-to-play
 * @param {Object} data - The unfurl metadata
 * @returns {string} - HTML string for YouTube card
 */
function renderYouTubeCard(data) {
    const videoId = data.embed_data?.video_id;
    if (!videoId) {
        return renderGenericCard(data);
    }

    const title = data.title ? escapeHtml(data.title) : 'YouTube Video';
    const thumbnailUrl = `https://img.youtube.com/vi/${escapeHtml(videoId)}/mqdefault.jpg`;

    return `<div class="unfurl-card unfurl-youtube" data-video-id="${escapeHtml(videoId)}">
        <div class="unfurl-youtube-thumb">
            <img src="${thumbnailUrl}" alt="${title}" loading="lazy" />
            <div class="unfurl-youtube-play" aria-label="Play video">
                <svg viewBox="0 0 68 48" width="68" height="48">
                    <path class="unfurl-youtube-play-bg" d="M66.52,7.74c-0.78-2.93-2.49-5.41-5.42-6.19C55.79,.13,34,0,34,0S12.21,.13,6.9,1.55 C3.97,2.33,2.27,4.81,1.48,7.74C0.06,13.05,0,24,0,24s0.06,10.95,1.48,16.26c0.78,2.93,2.49,5.41,5.42,6.19 C12.21,47.87,34,48,34,48s21.79-0.13,27.1-1.55c2.93-0.78,4.64-3.26,5.42-6.19C67.94,34.95,68,24,68,24S67.94,13.05,66.52,7.74z" fill="#f00"/>
                    <path d="M 45,24 27,14 27,34" fill="#fff"/>
                </svg>
            </div>
        </div>
        <div class="unfurl-content">
            <div class="unfurl-site">
                <svg class="unfurl-favicon" width="16" height="16" viewBox="0 0 24 24" fill="#f00">
                    <path d="M23.498 6.186a3.016 3.016 0 0 0-2.122-2.136C19.505 3.545 12 3.545 12 3.545s-7.505 0-9.377.505A3.017 3.017 0 0 0 .502 6.186C0 8.07 0 12 0 12s0 3.93.502 5.814a3.016 3.016 0 0 0 2.122 2.136c1.871.505 9.376.505 9.376.505s7.505 0 9.377-.505a3.015 3.015 0 0 0 2.122-2.136C24 15.93 24 12 24 12s0-3.93-.502-5.814zM9.545 15.568V8.432L15.818 12l-6.273 3.568z"/>
                </svg>
                <span class="unfurl-site-name">YouTube</span>
            </div>
            <div class="unfurl-title">${title}</div>
        </div>
    </div>`;
}

/**
 * Render Twitter/X card
 * @param {Object} data - The unfurl metadata
 * @returns {string} - HTML string for Twitter card
 */
function renderTwitterCard(data) {
    // For now, render as a styled generic card
    // Full Twitter embed would require loading Twitter's widget.js
    let html = '<div class="unfurl-card unfurl-twitter">';

    // Use Twitter/X branding
    html += '<div class="unfurl-content">';
    html += '<div class="unfurl-site">';
    html += `<svg class="unfurl-favicon" width="16" height="16" viewBox="0 0 24 24" fill="currentColor">
        <path d="M18.244 2.25h3.308l-7.227 8.26 8.502 11.24H16.17l-5.214-6.817L4.99 21.75H1.68l7.73-8.835L1.254 2.25H8.08l4.713 6.231zm-1.161 17.52h1.833L7.084 4.126H5.117z"/>
    </svg>`;
    html += '<span class="unfurl-site-name">X (Twitter)</span>';
    html += '</div>';

    if (data.title) {
        html += `<div class="unfurl-title">${escapeHtml(data.title)}</div>`;
    }

    if (data.description) {
        html += `<div class="unfurl-description">${escapeHtml(data.description)}</div>`;
    }

    if (data.image_url) {
        html += `<div class="unfurl-image"><img src="${escapeHtml(data.image_url)}" alt="" loading="lazy" /></div>`;
    }

    html += '</div></div>';
    return html;
}

/**
 * Render GitHub repository card
 * @param {Object} data - The unfurl metadata
 * @returns {string} - HTML string for GitHub card
 */
function renderGitHubCard(data) {
    const owner = data.embed_data?.repo_owner;
    const repo = data.embed_data?.repo_name;

    let html = '<div class="unfurl-card unfurl-github">';
    html += '<div class="unfurl-content">';

    // GitHub branding
    html += '<div class="unfurl-site">';
    html += `<svg class="unfurl-favicon" width="16" height="16" viewBox="0 0 16 16" fill="currentColor">
        <path d="M8 0C3.58 0 0 3.58 0 8c0 3.54 2.29 6.53 5.47 7.59.4.07.55-.17.55-.38 0-.19-.01-.82-.01-1.49-2.01.37-2.53-.49-2.69-.94-.09-.23-.48-.94-.82-1.13-.28-.15-.68-.52-.01-.53.63-.01 1.08.58 1.23.82.72 1.21 1.87.87 2.33.66.07-.52.28-.87.51-1.07-1.78-.2-3.64-.89-3.64-3.95 0-.87.31-1.59.82-2.15-.08-.2-.36-1.02.08-2.12 0 0 .67-.21 2.2.82.64-.18 1.32-.27 2-.27.68 0 1.36.09 2 .27 1.53-1.04 2.2-.82 2.2-.82.44 1.1.16 1.92.08 2.12.51.56.82 1.27.82 2.15 0 3.07-1.87 3.75-3.65 3.95.29.25.54.73.54 1.48 0 1.07-.01 1.93-.01 2.2 0 .21.15.46.55.38A8.013 8.013 0 0016 8c0-4.42-3.58-8-8-8z"/>
    </svg>`;
    html += '<span class="unfurl-site-name">GitHub</span>';
    html += '</div>';

    // Repo name
    if (owner && repo) {
        html += `<div class="unfurl-title unfurl-github-repo">${escapeHtml(owner)}/${escapeHtml(repo)}</div>`;
    } else if (data.title) {
        html += `<div class="unfurl-title">${escapeHtml(data.title)}</div>`;
    }

    // Description
    if (data.description) {
        html += `<div class="unfurl-description">${escapeHtml(data.description)}</div>`;
    }

    html += '</div></div>';
    return html;
}

/**
 * Render generic unfurl card (fallback)
 * @param {Object} data - The unfurl metadata
 * @returns {string} - HTML string for generic card
 */
function renderGenericCard(data) {
    if (!data.success) {
        return '';
    }

    let html = '<div class="unfurl-card">';

    // Image (if available)
    if (data.image_url) {
        html += `<div class="unfurl-image"><img src="${escapeHtml(data.image_url)}" alt="" loading="lazy" /></div>`;
    }

    html += '<div class="unfurl-content">';

    // Site name and favicon
    if (data.site_name || data.favicon_url) {
        html += '<div class="unfurl-site">';
        if (data.favicon_url) {
            html += `<img class="unfurl-favicon" src="${escapeHtml(data.favicon_url)}" alt="" width="16" height="16" />`;
        }
        if (data.site_name) {
            html += `<span class="unfurl-site-name">${escapeHtml(data.site_name)}</span>`;
        }
        html += '</div>';
    }

    // Title
    if (data.title) {
        html += `<div class="unfurl-title">${escapeHtml(data.title)}</div>`;
    }

    // Description
    if (data.description) {
        html += `<div class="unfurl-description">${escapeHtml(data.description)}</div>`;
    }

    html += '</div></div>';

    return html;
}

/**
 * Escape HTML special characters
 * @param {string} str - The string to escape
 * @returns {string} - The escaped string
 */
function escapeHtml(str) {
    const div = document.createElement('div');
    div.textContent = str;
    return div.innerHTML;
}

/**
 * Process a single unfurl container
 * @param {HTMLElement} container - The unfurl container element
 */
async function processUnfurlContainer(container) {
    const url = container.dataset.url;
    const previewEl = container.querySelector('.unfurl-preview');

    if (!url || !previewEl) {
        return;
    }

    // Skip if already processed
    if (container.dataset.unfurlProcessed === 'true') {
        return;
    }
    container.dataset.unfurlProcessed = 'true';

    try {
        const data = await fetchUnfurl(url);
        const cardHtml = renderUnfurlCard(data);

        if (cardHtml) {
            previewEl.innerHTML = cardHtml;
            previewEl.classList.remove('unfurl-loading');
            previewEl.classList.add('unfurl-loaded');
        } else {
            // No preview available, hide the preview element
            previewEl.style.display = 'none';
        }
    } catch (error) {
        console.error('Unfurl failed for URL:', url, error);
        // Hide the preview on error
        previewEl.style.display = 'none';
    }
}

/**
 * Process all unfurl containers on the page
 */
function processAllUnfurls() {
    const containers = document.querySelectorAll('.unfurl-container:not([data-unfurl-processed="true"])');
    containers.forEach(processUnfurlContainer);
}

/**
 * Initialize unfurl functionality
 */
function init() {
    // Process existing unfurl containers
    processAllUnfurls();

    // Watch for new unfurl containers (e.g., from AJAX or dynamic content)
    const observer = new MutationObserver((mutations) => {
        let shouldProcess = false;

        mutations.forEach((mutation) => {
            if (mutation.addedNodes.length > 0) {
                mutation.addedNodes.forEach((node) => {
                    if (node.nodeType === Node.ELEMENT_NODE) {
                        if (node.matches('.unfurl-container') ||
                            node.querySelector('.unfurl-container')) {
                            shouldProcess = true;
                        }
                    }
                });
            }
        });

        if (shouldProcess) {
            processAllUnfurls();
        }
    });

    observer.observe(document.body, {
        childList: true,
        subtree: true
    });

    // Click-to-play handler for YouTube videos
    document.addEventListener('click', (e) => {
        const thumb = e.target.closest('.unfurl-youtube-thumb');
        if (!thumb) return;

        const card = thumb.closest('.unfurl-youtube');
        if (!card || card.classList.contains('unfurl-youtube-playing')) return;

        e.preventDefault();
        const videoId = card.dataset.videoId;
        if (!videoId) return;

        // Create and insert iframe
        const iframe = document.createElement('iframe');
        iframe.src = `https://www.youtube-nocookie.com/embed/${videoId}?autoplay=1`;
        iframe.setAttribute('frameborder', '0');
        iframe.setAttribute('allowfullscreen', '');
        iframe.setAttribute('allow', 'accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture');
        iframe.className = 'unfurl-youtube-iframe';

        // Replace thumbnail with iframe
        thumb.style.display = 'none';
        card.insertBefore(iframe, thumb);
        card.classList.add('unfurl-youtube-playing');
    });
}

// Run on DOMContentLoaded
if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', init);
} else {
    init();
}

// Export for external use
window.RuforoUnfurl = {
    processAllUnfurls,
    fetchUnfurl
};
