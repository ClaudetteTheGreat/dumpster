/**
 * Lightbox for thumbnails and attachment images
 * Intercepts clicks on .bbcode-thumb and .attachment-preview (images only)
 * Displays full-size image in a modal overlay
 */

(function() {
    'use strict';

    let lightbox = null;
    let lightboxImg = null;
    let currentImages = [];
    let currentIndex = 0;

    /**
     * Create the lightbox DOM elements
     */
    function createLightbox() {
        if (lightbox) return;

        lightbox = document.createElement('div');
        lightbox.className = 'lightbox';
        lightbox.innerHTML = `
            <div class="lightbox-overlay"></div>
            <div class="lightbox-container">
                <button class="lightbox-close" aria-label="Close">&times;</button>
                <button class="lightbox-prev" aria-label="Previous">&lsaquo;</button>
                <button class="lightbox-next" aria-label="Next">&rsaquo;</button>
                <img class="lightbox-image" src="" alt="" />
            </div>
        `;

        document.body.appendChild(lightbox);

        lightboxImg = lightbox.querySelector('.lightbox-image');

        // Close on overlay click
        lightbox.querySelector('.lightbox-overlay').addEventListener('click', close);
        lightbox.querySelector('.lightbox-close').addEventListener('click', close);

        // Navigation
        lightbox.querySelector('.lightbox-prev').addEventListener('click', showPrev);
        lightbox.querySelector('.lightbox-next').addEventListener('click', showNext);

        // Keyboard navigation
        document.addEventListener('keydown', handleKeydown);
    }

    /**
     * Handle keyboard events
     */
    function handleKeydown(e) {
        if (!lightbox || !lightbox.classList.contains('lightbox--open')) return;

        switch (e.key) {
            case 'Escape':
                close();
                break;
            case 'ArrowLeft':
                showPrev();
                break;
            case 'ArrowRight':
                showNext();
                break;
        }
    }

    /**
     * Open lightbox with given image URL
     */
    function open(imageUrl, images, index) {
        createLightbox();

        currentImages = images || [imageUrl];
        currentIndex = index || 0;

        lightboxImg.src = currentImages[currentIndex];
        lightbox.classList.add('lightbox--open');
        document.body.style.overflow = 'hidden';

        updateNavButtons();
    }

    /**
     * Close the lightbox
     */
    function close() {
        if (!lightbox) return;

        lightbox.classList.remove('lightbox--open');
        document.body.style.overflow = '';
        lightboxImg.src = '';
    }

    /**
     * Show previous image
     */
    function showPrev() {
        if (currentImages.length <= 1) return;

        currentIndex = (currentIndex - 1 + currentImages.length) % currentImages.length;
        lightboxImg.src = currentImages[currentIndex];
        updateNavButtons();
    }

    /**
     * Show next image
     */
    function showNext() {
        if (currentImages.length <= 1) return;

        currentIndex = (currentIndex + 1) % currentImages.length;
        lightboxImg.src = currentImages[currentIndex];
        updateNavButtons();
    }

    /**
     * Update navigation button visibility
     */
    function updateNavButtons() {
        const prevBtn = lightbox.querySelector('.lightbox-prev');
        const nextBtn = lightbox.querySelector('.lightbox-next');

        if (currentImages.length <= 1) {
            prevBtn.style.display = 'none';
            nextBtn.style.display = 'none';
        } else {
            prevBtn.style.display = '';
            nextBtn.style.display = '';
        }
    }

    /**
     * Collect all images in the current context (post/message)
     */
    function collectImagesInContext(clickedElement) {
        // Find the parent post/message container
        const container = clickedElement.closest('.message, .post, .ugc, .message-attachments');
        if (!container) {
            return [clickedElement.href];
        }

        // Collect all lightbox-eligible images in this container
        const images = [];
        const selectors = '.bbcode-thumb, .attachment-preview';
        const links = container.querySelectorAll(selectors);

        links.forEach(link => {
            // Only include image attachments
            if (link.classList.contains('attachment-preview')) {
                const img = link.querySelector('img');
                if (!img) return; // Skip non-image attachments
            }
            images.push(link.href);
        });

        return images;
    }

    /**
     * Handle click on thumbnail or attachment
     */
    function handleImageClick(e) {
        const link = e.target.closest('.bbcode-thumb, .attachment-preview');
        if (!link) return;

        // For attachment-preview, only handle if it contains an image
        if (link.classList.contains('attachment-preview')) {
            const img = link.querySelector('img');
            if (!img) return; // Let non-image attachments open normally
        }

        e.preventDefault();

        const images = collectImagesInContext(link);
        const index = images.indexOf(link.href);

        open(link.href, images, index >= 0 ? index : 0);
    }

    /**
     * Initialize lightbox functionality
     */
    function init() {
        // Use event delegation on document body
        document.body.addEventListener('click', handleImageClick);
    }

    // Initialize on DOM ready
    if (document.readyState === 'loading') {
        document.addEventListener('DOMContentLoaded', init);
    } else {
        init();
    }

    // Expose for external use
    window.RuforoLightbox = {
        open,
        close
    };

})();
