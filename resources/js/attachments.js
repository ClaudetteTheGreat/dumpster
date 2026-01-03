document.addEventListener("DOMContentLoaded", function () {
    function attachmentEventListeners() {
        const inputEl = document.querySelector('.attachment-input');
        const previewsContainer = document.querySelector('.attachment-previews');
        const uploadBtn = document.querySelector('.attachment-upload');

        if (!inputEl || !previewsContainer) return;

        // Find the textarea dynamically (handles WYSIWYG mode changes)
        function getTextarea() {
            return document.querySelector('#reply-textarea, textarea[name="content"]');
        }

        // Track uploaded files with their server responses
        let uploadedFiles = [];

        // Create preview element for a file
        function createPreview(fileData, index) {
            const { file, uploadResponse, uploading } = fileData;
            const previewEl = document.createElement('div');
            previewEl.className = 'attachment-preview';
            previewEl.dataset.index = index;

            // Create remove button
            const removeBtn = document.createElement('button');
            removeBtn.type = 'button';
            removeBtn.className = 'attachment-remove';
            removeBtn.title = 'Remove';
            removeBtn.textContent = '×';
            removeBtn.addEventListener('click', function (e) {
                e.preventDefault();
                e.stopPropagation();
                removeFile(index);
            });
            previewEl.appendChild(removeBtn);

            // Create thumbnail or file icon
            if (file.type.startsWith('image/')) {
                const thumbnailEl = document.createElement('img');
                thumbnailEl.className = 'attachment-thumbnail';
                thumbnailEl.alt = 'Preview';
                const reader = new FileReader();
                reader.onload = function (e) {
                    thumbnailEl.src = e.target.result;
                };
                reader.readAsDataURL(file);
                previewEl.appendChild(thumbnailEl);

                // Add insert button for images (always add if uploaded)
                if (uploadResponse) {
                    const insertBtn = document.createElement('button');
                    insertBtn.type = 'button';
                    insertBtn.className = 'attachment-insert';
                    insertBtn.title = 'Insert into post';
                    insertBtn.textContent = 'Insert';
                    insertBtn.addEventListener('click', function (e) {
                        e.preventDefault();
                        e.stopPropagation();
                        insertIntoEditor(uploadResponse, file.name);
                    });
                    previewEl.appendChild(insertBtn);
                }
            } else {
                const iconEl = document.createElement('div');
                iconEl.className = 'attachment-file-icon';
                iconEl.textContent = getFileIcon(file.type);
                previewEl.appendChild(iconEl);
            }

            // Create filename
            const filenameEl = document.createElement('span');
            filenameEl.className = 'attachment-filename';
            filenameEl.textContent = file.name;
            filenameEl.title = file.name;
            previewEl.appendChild(filenameEl);

            // Show upload status
            if (uploading) {
                previewEl.classList.add('uploading');
            }

            return previewEl;
        }

        // Insert image BBCode into editor
        function insertIntoEditor(uploadResponse, originalFilename) {
            if (!uploadResponse || !uploadResponse.hash) {
                return;
            }

            const url = `/content/${uploadResponse.hash}/${encodeURIComponent(originalFilename)}`;
            const bbcode = `[img]${url}[/img]`;

            // Use the global insertEditorContent function if available
            if (typeof window.insertEditorContent === 'function') {
                window.insertEditorContent('reply-textarea', bbcode);
                return;
            }

            // Fallback: insert directly into textarea
            const textarea = getTextarea();
            if (textarea) {
                const start = textarea.selectionStart;
                const end = textarea.selectionEnd;
                const text = textarea.value;
                textarea.value = text.substring(0, start) + bbcode + text.substring(end);
                textarea.selectionStart = textarea.selectionEnd = start + bbcode.length;
                textarea.focus();
                textarea.dispatchEvent(new Event('input', { bubbles: true }));
            }
        }

        // Get icon for file type
        function getFileIcon(mimeType) {
            if (mimeType.startsWith('video/')) return '🎬';
            if (mimeType.startsWith('audio/')) return '🎵';
            if (mimeType === 'application/pdf') return '📄';
            if (mimeType.includes('zip') || mimeType.includes('rar') || mimeType.includes('7z')) return '📦';
            return '📎';
        }

        // Remove file by index
        function removeFile(index) {
            uploadedFiles.splice(index, 1);
            updateFileInput();
            refreshPreviews();
        }

        // Update the file input with current files
        function updateFileInput() {
            const newFileList = new DataTransfer();
            for (const fileData of uploadedFiles) {
                newFileList.items.add(fileData.file);
            }
            inputEl.files = newFileList.files;
        }

        // Refresh all previews
        function refreshPreviews() {
            previewsContainer.innerHTML = '';
            for (let i = 0; i < uploadedFiles.length; i++) {
                previewsContainer.appendChild(createPreview(uploadedFiles[i], i));
            }
        }

        // Upload a file to the server
        async function uploadFile(file) {
            const formData = new FormData();
            formData.append('file', file);

            try {
                const response = await fetch('/fs/upload-file', {
                    method: 'POST',
                    body: formData,
                });

                if (response.ok) {
                    const results = await response.json();
                    if (results.length > 0) {
                        return results[0];
                    }
                }
            } catch (err) {
                // Upload failed silently
            }
            return null;
        }

        // Handle file selection
        inputEl.addEventListener('change', async function (event) {
            const newFiles = event.target.files;

            // Add new files to our list and start uploading
            for (let i = 0; i < newFiles.length; i++) {
                const file = newFiles[i];
                const fileData = { file, uploadResponse: null, uploading: true };
                uploadedFiles.push(fileData);
                refreshPreviews();

                // Upload in background
                const response = await uploadFile(file);
                fileData.uploadResponse = response;
                fileData.uploading = false;
                refreshPreviews();
            }

            updateFileInput();
        });

        // Click "Attach File" button triggers file input
        if (uploadBtn) {
            uploadBtn.addEventListener('click', function (event) {
                event.preventDefault();
                inputEl.click();
            });
        }
    }

    attachmentEventListeners();

    // Reply button scroll handler (moved from inline onclick for CSP compliance)
    const scrollToReplyBtn = document.querySelector('.scroll-to-reply');
    if (scrollToReplyBtn) {
        scrollToReplyBtn.addEventListener('click', function() {
            const replyForm = document.getElementById('reply-form');
            const replyTextarea = document.getElementById('reply-textarea');
            if (replyForm) {
                replyForm.scrollIntoView({ behavior: 'smooth' });
                if (replyTextarea) {
                    setTimeout(() => replyTextarea.focus(), 500);
                }
            }
        });
    }
});
