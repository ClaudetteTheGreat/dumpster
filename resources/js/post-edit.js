/**
 * Inline Post Editing
 *
 * Handles toggling between view and edit modes for posts.
 */

document.addEventListener('DOMContentLoaded', function() {
    // Use event delegation for edit buttons
    document.addEventListener('click', function(e) {
        // Handle Edit button click
        if (e.target.classList.contains('edit-post-btn')) {
            const postId = e.target.dataset.postId;
            toggleEditMode(postId, true);
        }

        // Handle Cancel button click
        if (e.target.classList.contains('edit-cancel-btn')) {
            const editForm = e.target.closest('.message-edit-form');
            if (editForm) {
                const postId = editForm.dataset.postId;
                toggleEditMode(postId, false);
            }
        }
    });
});

/**
 * Toggle between view and edit mode for a post
 * @param {string} postId - The post ID
 * @param {boolean} showEdit - Whether to show edit mode (true) or view mode (false)
 */
function toggleEditMode(postId, showEdit) {
    const contentDiv = document.querySelector(`.message-content[data-post-id="${postId}"]`);
    const editForm = document.querySelector(`.message-edit-form[data-post-id="${postId}"]`);
    const editBtn = document.querySelector(`.edit-post-btn[data-post-id="${postId}"]`);

    if (!contentDiv || !editForm) {
        return;
    }

    if (showEdit) {
        // Show edit form, hide content
        contentDiv.style.display = 'none';
        editForm.style.display = 'block';

        // Hide the edit button while editing
        if (editBtn) {
            editBtn.style.display = 'none';
        }

        // Focus the textarea
        const textarea = editForm.querySelector('textarea');
        if (textarea) {
            textarea.focus();
            // Move cursor to end
            textarea.setSelectionRange(textarea.value.length, textarea.value.length);
        }
    } else {
        // Show content, hide edit form
        contentDiv.style.display = '';
        editForm.style.display = 'none';

        // Show the edit button again
        if (editBtn) {
            editBtn.style.display = '';
        }
    }
}
