/**
 * Character Counter for Post/Thread Forms
 *
 * Provides real-time character counting for textareas with visual feedback
 * based on remaining characters.
 */

document.addEventListener('DOMContentLoaded', () => {
    // Find all textareas with the char-counter data attribute
    const textareas = document.querySelectorAll('textarea[data-char-limit]');

    textareas.forEach(textarea => {
        const maxLength = parseInt(textarea.dataset.charLimit, 10);

        // Create counter element
        const counterDiv = document.createElement('div');
        counterDiv.className = 'char-counter';
        counterDiv.innerHTML = `<span class="char-count">0</span> / <span class="char-limit">${maxLength.toLocaleString()}</span> characters`;

        // Insert counter after textarea
        textarea.parentNode.insertBefore(counterDiv, textarea.nextSibling);

        const countSpan = counterDiv.querySelector('.char-count');

        // Update counter function
        const updateCounter = () => {
            const currentLength = textarea.value.length;
            const remaining = maxLength - currentLength;
            const percentUsed = (currentLength / maxLength) * 100;

            countSpan.textContent = currentLength.toLocaleString();

            // Remove all status classes
            counterDiv.classList.remove('char-counter--ok', 'char-counter--warning', 'char-counter--danger', 'char-counter--over');

            // Apply appropriate status class based on usage
            if (currentLength > maxLength) {
                counterDiv.classList.add('char-counter--over');
                textarea.classList.add('textarea--over-limit');
            } else {
                textarea.classList.remove('textarea--over-limit');

                if (percentUsed >= 95) {
                    counterDiv.classList.add('char-counter--danger');
                } else if (percentUsed >= 80) {
                    counterDiv.classList.add('char-counter--warning');
                } else {
                    counterDiv.classList.add('char-counter--ok');
                }
            }
        };

        // Attach event listeners
        textarea.addEventListener('input', updateCounter);
        textarea.addEventListener('change', updateCounter);

        // Initial update
        updateCounter();

        // Prevent form submission if over limit
        const form = textarea.closest('form');
        if (form) {
            form.addEventListener('submit', (e) => {
                if (textarea.value.length > maxLength) {
                    e.preventDefault();
                    alert(`Post is too long. Maximum length is ${maxLength.toLocaleString()} characters, but your post is ${textarea.value.length.toLocaleString()} characters.`);
                }
            });
        }
    });
});
