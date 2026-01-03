import { blake3 } from 'hash-wasm';

document.addEventListener("DOMContentLoaded", function () {
    function attachmentEventListeners() {
        const inputEl = document.querySelector('.attachment-input');
        const previewEl = document.querySelector('.attachment-preview');
        const thumbnailEl = document.querySelector('.attachment-thumbnail');
        const filenameEl = document.querySelector('.attachment-filename');
        const removeEl = document.querySelector('.attachment-remove');
        const uploadBtn = document.querySelector('.attachment-upload');

        if (!inputEl) return;

        // Show thumbnail preview when file is selected
        inputEl.addEventListener('change', async function (event) {
            const file = event.target.files[0];
            if (file) {
                // Show preview container
                if (previewEl) {
                    previewEl.style.display = 'flex';
                }

                // Set filename
                if (filenameEl) {
                    filenameEl.textContent = file.name;
                }

                // Show thumbnail for images
                if (thumbnailEl) {
                    if (file.type.startsWith('image/')) {
                        const reader = new FileReader();
                        reader.onload = function (e) {
                            thumbnailEl.src = e.target.result;
                            thumbnailEl.style.display = 'block';
                        };
                        reader.readAsDataURL(file);
                    } else {
                        // Show file type icon for non-images
                        thumbnailEl.style.display = 'none';
                    }
                }

                // Hash check (optional dedup)
                try {
                    const arrayBuffer = await file.arrayBuffer();
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
        });

        // Remove button clears the file
        if (removeEl) {
            removeEl.addEventListener('click', function () {
                inputEl.value = '';
                if (previewEl) {
                    previewEl.style.display = 'none';
                }
                if (thumbnailEl) {
                    thumbnailEl.src = '';
                }
                if (filenameEl) {
                    filenameEl.textContent = '';
                }
            });
        }

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
