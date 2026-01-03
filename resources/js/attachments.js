import { blake3 } from 'hash-wasm';

document.addEventListener("DOMContentLoaded", function () {
    function attachmentEventListeners() {
        const inputEl = document.querySelector('.attachment-input');
        const previewsContainer = document.querySelector('.attachment-previews');
        const uploadBtn = document.querySelector('.attachment-upload');

        if (!inputEl || !previewsContainer) return;

        // Track selected files using DataTransfer
        let fileList = new DataTransfer();

        // Create preview element for a file
        function createPreview(file, index) {
            const previewEl = document.createElement('div');
            previewEl.className = 'attachment-preview';
            previewEl.dataset.index = index;

            // Create remove button
            const removeBtn = document.createElement('button');
            removeBtn.type = 'button';
            removeBtn.className = 'attachment-remove';
            removeBtn.title = 'Remove';
            removeBtn.textContent = '×';
            removeBtn.addEventListener('click', function () {
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

            return previewEl;
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
            const newFileList = new DataTransfer();
            for (let i = 0; i < fileList.files.length; i++) {
                if (i !== index) {
                    newFileList.items.add(fileList.files[i]);
                }
            }
            fileList = newFileList;
            inputEl.files = fileList.files;
            refreshPreviews();
        }

        // Refresh all previews
        function refreshPreviews() {
            previewsContainer.innerHTML = '';
            for (let i = 0; i < fileList.files.length; i++) {
                previewsContainer.appendChild(createPreview(fileList.files[i], i));
            }
        }

        // Handle file selection
        inputEl.addEventListener('change', async function (event) {
            const newFiles = event.target.files;

            // Add new files to our list
            for (let i = 0; i < newFiles.length; i++) {
                fileList.items.add(newFiles[i]);

                // Hash check (optional dedup)
                try {
                    const arrayBuffer = await newFiles[i].arrayBuffer();
                    const hash = await blake3(new Uint8Array(arrayBuffer));

                    const response = await fetch('/fs/check-file', {
                        method: "POST",
                        headers: {
                            'Content-Type': 'application/json'
                        },
                        body: JSON.stringify({ hash }),
                    });
                    console.log('File hash check:', response.status);
                } catch (err) {
                    console.log("Error checking file hash:", err);
                }
            }

            // Update input with combined file list
            inputEl.files = fileList.files;
            refreshPreviews();
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
});
