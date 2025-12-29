/**
 * Report Modal Functionality
 * Handles the report button and modal for reporting posts, threads, users
 */

document.addEventListener('DOMContentLoaded', function() {
    // Create modal HTML
    const modalHtml = `
        <div id="report-modal" class="report-modal" style="display: none;">
            <div class="report-modal-overlay"></div>
            <div class="report-modal-content">
                <div class="report-modal-header">
                    <h3>Report Content</h3>
                    <button type="button" class="report-modal-close">&times;</button>
                </div>
                <form id="report-form" class="report-form">
                    <input type="hidden" name="csrf_token" id="report-csrf" />
                    <input type="hidden" name="content_type" id="report-content-type" />
                    <input type="hidden" name="content_id" id="report-content-id" />

                    <div class="form-group">
                        <label for="report-reason">Reason</label>
                        <select name="reason" id="report-reason" required>
                            <option value="">Select a reason...</option>
                        </select>
                    </div>

                    <div class="form-group" id="details-group">
                        <label for="report-details">Details <span id="details-required">(optional)</span></label>
                        <textarea name="details" id="report-details" rows="4"
                            placeholder="Provide additional context about this report..."></textarea>
                    </div>

                    <div class="form-actions">
                        <button type="button" class="btn btn-secondary report-modal-cancel">Cancel</button>
                        <button type="submit" class="btn btn-danger" id="report-submit">Submit Report</button>
                    </div>

                    <div id="report-message" class="report-message" style="display: none;"></div>
                </form>
            </div>
        </div>
    `;

    // Add modal to body
    document.body.insertAdjacentHTML('beforeend', modalHtml);

    const modal = document.getElementById('report-modal');
    const form = document.getElementById('report-form');
    const reasonSelect = document.getElementById('report-reason');
    const detailsGroup = document.getElementById('details-group');
    const detailsRequired = document.getElementById('details-required');
    const detailsInput = document.getElementById('report-details');
    const messageDiv = document.getElementById('report-message');
    const submitBtn = document.getElementById('report-submit');

    let reportReasons = [];

    // Fetch report reasons when page loads
    async function fetchReportReasons() {
        try {
            const response = await fetch('/api/report-reasons');
            if (response.ok) {
                reportReasons = await response.json();
                populateReasons();
            }
        } catch (error) {
            console.error('Failed to fetch report reasons:', error);
        }
    }

    function populateReasons() {
        reasonSelect.innerHTML = '<option value="">Select a reason...</option>';
        reportReasons.forEach(reason => {
            const option = document.createElement('option');
            option.value = reason.name;
            option.textContent = reason.label;
            if (reason.description) {
                option.title = reason.description;
            }
            reasonSelect.appendChild(option);
        });
    }

    // Handle reason change - show details required for "other"
    reasonSelect.addEventListener('change', function() {
        if (this.value === 'other') {
            detailsRequired.textContent = '(required)';
            detailsInput.required = true;
        } else {
            detailsRequired.textContent = '(optional)';
            detailsInput.required = false;
        }
    });

    // Open modal
    function openModal(contentType, contentId, csrfToken) {
        document.getElementById('report-csrf').value = csrfToken;
        document.getElementById('report-content-type').value = contentType;
        document.getElementById('report-content-id').value = contentId;

        // Reset form
        form.reset();
        reasonSelect.value = '';
        detailsRequired.textContent = '(optional)';
        detailsInput.required = false;
        messageDiv.style.display = 'none';
        submitBtn.disabled = false;
        submitBtn.textContent = 'Submit Report';

        modal.style.display = 'flex';
        document.body.style.overflow = 'hidden';

        // Fetch reasons if not loaded
        if (reportReasons.length === 0) {
            fetchReportReasons();
        }
    }

    // Close modal
    function closeModal() {
        modal.style.display = 'none';
        document.body.style.overflow = '';
    }

    // Handle report button clicks
    document.addEventListener('click', function(e) {
        const reportBtn = e.target.closest('.report-btn');
        if (reportBtn) {
            const contentType = reportBtn.dataset.contentType;
            const contentId = reportBtn.dataset.contentId;
            const csrfToken = reportBtn.dataset.csrf;
            openModal(contentType, contentId, csrfToken);
        }
    });

    // Close modal events
    modal.querySelector('.report-modal-overlay').addEventListener('click', closeModal);
    modal.querySelector('.report-modal-close').addEventListener('click', closeModal);
    modal.querySelector('.report-modal-cancel').addEventListener('click', closeModal);

    document.addEventListener('keydown', function(e) {
        if (e.key === 'Escape' && modal.style.display !== 'none') {
            closeModal();
        }
    });

    // Handle form submission
    form.addEventListener('submit', async function(e) {
        e.preventDefault();

        submitBtn.disabled = true;
        submitBtn.textContent = 'Submitting...';
        messageDiv.style.display = 'none';

        // Send as URL-encoded form data (not multipart)
        const formData = new URLSearchParams(new FormData(form));

        try {
            const response = await fetch('/reports', {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/x-www-form-urlencoded',
                },
                body: formData
            });

            const result = await response.json();

            if (result.success) {
                messageDiv.className = 'report-message report-success';
                messageDiv.textContent = result.message;
                messageDiv.style.display = 'block';

                // Close modal after delay
                setTimeout(closeModal, 2000);
            } else {
                messageDiv.className = 'report-message report-error';
                messageDiv.textContent = result.message;
                messageDiv.style.display = 'block';
                submitBtn.disabled = false;
                submitBtn.textContent = 'Submit Report';
            }
        } catch (error) {
            messageDiv.className = 'report-message report-error';
            messageDiv.textContent = 'An error occurred. Please try again.';
            messageDiv.style.display = 'block';
            submitBtn.disabled = false;
            submitBtn.textContent = 'Submit Report';
        }
    });

    // Pre-fetch report reasons
    fetchReportReasons();
});
