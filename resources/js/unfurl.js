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
 * Render unfurl preview card
 * @param {Object} data - The unfurl metadata
 * @returns {string} - HTML string for the preview card
 */
function renderUnfurlCard(data) {
    if (!data.success) {
        return ''; // Don't show anything if unfurl failed
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
