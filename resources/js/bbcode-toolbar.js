/**
 * BBCode Toolbar
 * Provides formatting buttons for BBCode text editing
 */

document.addEventListener('DOMContentLoaded', function() {
    // Find all textareas with bbcode-editor class or within bbcode-editor-container
    const editors = document.querySelectorAll('.bbcode-editor-container');

    editors.forEach(container => {
        const textarea = container.querySelector('textarea');
        const toolbar = container.querySelector('.bbcode-toolbar');

        if (!textarea || !toolbar) return;

        // Set up toolbar button handlers
        toolbar.querySelectorAll('[data-bbcode]').forEach(button => {
            button.addEventListener('click', (e) => {
                e.preventDefault();
                const tag = button.dataset.bbcode;
                const param = button.dataset.param || '';
                insertBBCode(textarea, tag, param);
            });
        });

        // Set up special buttons
        toolbar.querySelectorAll('[data-action]').forEach(button => {
            button.addEventListener('click', (e) => {
                e.preventDefault();
                handleAction(textarea, button.dataset.action, container);
            });
        });
    });

    /**
     * Insert BBCode tags around selected text or at cursor
     */
    function insertBBCode(textarea, tag, param = '') {
        const start = textarea.selectionStart;
        const end = textarea.selectionEnd;
        const selectedText = textarea.value.substring(start, end);
        const beforeText = textarea.value.substring(0, start);
        const afterText = textarea.value.substring(end);

        let openTag, closeTag;

        // Handle special cases
        switch (tag) {
            case 'url':
                if (selectedText) {
                    // If text is selected, prompt for URL
                    const url = prompt('Enter URL:', 'https://');
                    if (url === null) return;
                    openTag = `[url=${url}]`;
                    closeTag = '[/url]';
                } else {
                    // If no text selected, just insert [url][/url]
                    openTag = '[url]';
                    closeTag = '[/url]';
                }
                break;

            case 'img':
                if (!selectedText) {
                    const imgUrl = prompt('Enter image URL:', 'https://');
                    if (imgUrl === null) return;
                    textarea.value = beforeText + `[img]${imgUrl}[/img]` + afterText;
                    textarea.selectionStart = textarea.selectionEnd = start + 5 + imgUrl.length + 6;
                    textarea.focus();
                    triggerInput(textarea);
                    return;
                }
                openTag = '[img]';
                closeTag = '[/img]';
                break;

            case 'color':
                const color = param || prompt('Enter color (name or #hex):', 'red');
                if (color === null) return;
                openTag = `[color=${color}]`;
                closeTag = '[/color]';
                break;

            case 'size':
                const size = param || prompt('Enter size (8-36):', '14');
                if (size === null) return;
                openTag = `[size=${size}]`;
                closeTag = '[/size]';
                break;

            case 'quote':
                if (param) {
                    openTag = `[quote=${param}]`;
                } else {
                    const author = prompt('Quote author (leave empty for no attribution):');
                    if (author === null) return;
                    openTag = author ? `[quote=${author}]` : '[quote]';
                }
                closeTag = '[/quote]';
                break;

            case 'spoiler':
                const spoilerTitle = prompt('Spoiler title (leave empty for default):');
                if (spoilerTitle === null) return;
                openTag = spoilerTitle ? `[spoiler=${spoilerTitle}]` : '[spoiler]';
                closeTag = '[/spoiler]';
                break;

            case 'video':
                if (!selectedText) {
                    const videoUrl = prompt('Enter video URL (YouTube, Vimeo, or direct video):', 'https://');
                    if (videoUrl === null) return;
                    textarea.value = beforeText + `[video]${videoUrl}[/video]` + afterText;
                    textarea.selectionStart = textarea.selectionEnd = start + 7 + videoUrl.length + 8;
                    textarea.focus();
                    triggerInput(textarea);
                    return;
                }
                openTag = '[video]';
                closeTag = '[/video]';
                break;

            case 'audio':
                if (!selectedText) {
                    const audioUrl = prompt('Enter audio URL:', 'https://');
                    if (audioUrl === null) return;
                    textarea.value = beforeText + `[audio]${audioUrl}[/audio]` + afterText;
                    textarea.selectionStart = textarea.selectionEnd = start + 7 + audioUrl.length + 8;
                    textarea.focus();
                    triggerInput(textarea);
                    return;
                }
                openTag = '[audio]';
                closeTag = '[/audio]';
                break;

            case 'youtube':
                if (!selectedText) {
                    const ytUrl = prompt('Enter YouTube video URL or ID:', '');
                    if (ytUrl === null) return;
                    textarea.value = beforeText + `[youtube]${ytUrl}[/youtube]` + afterText;
                    textarea.selectionStart = textarea.selectionEnd = start + 9 + ytUrl.length + 10;
                    textarea.focus();
                    triggerInput(textarea);
                    return;
                }
                openTag = '[youtube]';
                closeTag = '[/youtube]';
                break;

            case 'list':
                // Insert a list template
                if (!selectedText) {
                    const listContent = '[list]\n[*] Item 1\n[*] Item 2\n[*] Item 3\n[/list]';
                    textarea.value = beforeText + listContent + afterText;
                    textarea.selectionStart = start + 11;
                    textarea.selectionEnd = start + 17;
                    textarea.focus();
                    triggerInput(textarea);
                    return;
                }
                // Wrap selected text as list items
                const items = selectedText.split('\n').filter(line => line.trim());
                const listItems = items.map(item => `[*] ${item.trim()}`).join('\n');
                textarea.value = beforeText + `[list]\n${listItems}\n[/list]` + afterText;
                textarea.selectionStart = textarea.selectionEnd = beforeText.length + 7 + listItems.length + 8;
                textarea.focus();
                triggerInput(textarea);
                return;

            default:
                // Simple tags like [b], [i], [u], [s], [code], [center], etc.
                openTag = `[${tag}]`;
                closeTag = `[/${tag}]`;
        }

        // Insert the tags
        const newText = openTag + selectedText + closeTag;
        textarea.value = beforeText + newText + afterText;

        // Position cursor appropriately
        if (selectedText) {
            // Select the wrapped text
            textarea.selectionStart = start + openTag.length;
            textarea.selectionEnd = start + openTag.length + selectedText.length;
        } else {
            // Position cursor between tags
            textarea.selectionStart = textarea.selectionEnd = start + openTag.length;
        }

        textarea.focus();
        triggerInput(textarea);
    }

    /**
     * Handle special toolbar actions
     */
    function handleAction(textarea, action, container) {
        switch (action) {
            case 'preview':
                togglePreview(textarea, container);
                break;
        }
    }

    /**
     * Toggle preview mode with server-side BBCode rendering
     */
    async function togglePreview(textarea, container) {
        let preview = container.querySelector('.bbcode-preview');
        const previewBtn = container.querySelector('.preview-btn');

        if (preview) {
            // Hide preview, show textarea
            preview.remove();
            textarea.style.display = '';
            if (previewBtn) {
                previewBtn.textContent = 'Preview';
                previewBtn.classList.remove('preview-btn--active');
            }
            return;
        }

        // Show loading state
        if (previewBtn) {
            previewBtn.textContent = 'Loading...';
            previewBtn.disabled = true;
        }

        try {
            // Fetch rendered BBCode from server
            const response = await fetch('/api/bbcode/preview', {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/json',
                },
                body: JSON.stringify({ content: textarea.value }),
            });

            if (!response.ok) {
                throw new Error('Preview failed');
            }

            const html = await response.text();

            // Create preview element
            preview = document.createElement('div');
            preview.className = 'bbcode-preview';
            preview.innerHTML = '<div class="preview-header">Preview <span class="preview-edit-hint">(click Edit to continue editing)</span></div>' +
                '<div class="preview-content ugc">' + html + '</div>';

            // Hide textarea, show preview
            textarea.style.display = 'none';
            textarea.parentNode.insertBefore(preview, textarea.nextSibling);

            if (previewBtn) {
                previewBtn.textContent = 'Edit';
                previewBtn.classList.add('preview-btn--active');
            }
        } catch (error) {
            console.error('Preview error:', error);
            // Fallback to showing raw content
            preview = document.createElement('div');
            preview.className = 'bbcode-preview';
            preview.innerHTML = '<div class="preview-header preview-header--error">Preview unavailable</div>' +
                '<div class="preview-content">' + escapeHtml(textarea.value) + '</div>';

            textarea.style.display = 'none';
            textarea.parentNode.insertBefore(preview, textarea.nextSibling);

            if (previewBtn) {
                previewBtn.textContent = 'Edit';
                previewBtn.classList.add('preview-btn--active');
            }
        } finally {
            if (previewBtn) {
                previewBtn.disabled = false;
            }
        }
    }

    /**
     * Escape HTML for safe display
     */
    function escapeHtml(text) {
        const div = document.createElement('div');
        div.textContent = text;
        return div.innerHTML;
    }

    /**
     * Trigger input event to update character counter
     */
    function triggerInput(textarea) {
        textarea.dispatchEvent(new Event('input', { bubbles: true }));
    }
});

/**
 * Helper function to wrap selected text with BBCode (for external use)
 */
window.insertBBCodeTag = function(textareaId, tag, param) {
    const textarea = document.getElementById(textareaId);
    if (!textarea) return;

    const container = textarea.closest('.bbcode-editor-container');
    if (container) {
        const button = container.querySelector(`[data-bbcode="${tag}"]`);
        if (button) {
            button.click();
            return;
        }
    }

    // Fallback: manual insert
    const start = textarea.selectionStart;
    const end = textarea.selectionEnd;
    const selectedText = textarea.value.substring(start, end);
    const beforeText = textarea.value.substring(0, start);
    const afterText = textarea.value.substring(end);

    const openTag = param ? `[${tag}=${param}]` : `[${tag}]`;
    const closeTag = `[/${tag}]`;

    textarea.value = beforeText + openTag + selectedText + closeTag + afterText;
    textarea.selectionStart = start + openTag.length;
    textarea.selectionEnd = start + openTag.length + selectedText.length;
    textarea.focus();
};
