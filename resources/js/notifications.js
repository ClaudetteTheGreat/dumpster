/**
 * Real-time notification WebSocket client
 *
 * Connects to /notifications.ws and handles incoming notifications:
 * - Updates the notification badge count
 * - Shows toast notifications for new notifications
 */

document.addEventListener("DOMContentLoaded", function() {
    // Only connect if user is logged in
    if (typeof window.RUFORO_USER === 'undefined' || !window.RUFORO_USER.id) {
        return;
    }

    let ws = null;
    let reconnectTimer = null;
    let reconnectAttempts = 0;
    const MAX_RECONNECT_ATTEMPTS = 10;
    const RECONNECT_DELAY_BASE = 1000; // Start with 1 second
    const TOAST_DURATION = 5000; // 5 seconds

    /**
     * Connect to the notification WebSocket
     */
    function connect() {
        // Build WebSocket URL
        const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
        const wsUrl = `${protocol}//${window.location.host}/notifications.ws`;

        try {
            ws = new WebSocket(wsUrl);
        } catch (err) {
            console.error('Failed to create WebSocket:', err);
            scheduleReconnect();
            return;
        }

        ws.addEventListener('open', function() {
            console.log('Notification WebSocket connected');
            reconnectAttempts = 0;
        });

        ws.addEventListener('message', function(event) {
            handleMessage(event.data);
        });

        ws.addEventListener('close', function(event) {
            console.log('Notification WebSocket closed:', event.code, event.reason);
            scheduleReconnect();
        });

        ws.addEventListener('error', function(event) {
            console.error('Notification WebSocket error:', event);
        });
    }

    /**
     * Schedule a reconnection attempt with exponential backoff
     */
    function scheduleReconnect() {
        if (reconnectTimer) {
            clearTimeout(reconnectTimer);
        }

        if (reconnectAttempts >= MAX_RECONNECT_ATTEMPTS) {
            console.log('Max reconnection attempts reached');
            return;
        }

        const delay = RECONNECT_DELAY_BASE * Math.pow(2, reconnectAttempts);
        reconnectAttempts++;

        console.log(`Scheduling reconnect in ${delay}ms (attempt ${reconnectAttempts})`);
        reconnectTimer = setTimeout(connect, delay);
    }

    /**
     * Handle incoming WebSocket message
     */
    function handleMessage(data) {
        let json;
        try {
            json = JSON.parse(data);
        } catch (err) {
            console.error('Failed to parse notification message:', err);
            return;
        }

        if (json.type === 'notification' && json.data) {
            handleNotification(json.data);
        } else if (json.type === 'pong') {
            // Keep-alive response, ignore
        }
    }

    /**
     * Handle a new notification
     */
    function handleNotification(notification) {
        // Update badge count
        updateBadge();

        // Show toast notification
        showToast(notification);

        // Play notification sound if enabled (future feature)
        // playNotificationSound();
    }

    /**
     * Update the notification badge count
     */
    function updateBadge() {
        const badge = document.getElementById('notification-badge');
        if (!badge) return;

        let count = parseInt(badge.textContent, 10) || 0;
        count++;

        badge.textContent = count;
        badge.setAttribute('aria-label', `${count} unread notifications`);
        badge.classList.remove('hidden');
    }

    /**
     * Show a toast notification
     */
    function showToast(notification) {
        const container = document.getElementById('notification-toasts');
        if (!container) return;

        const toast = document.createElement('div');
        toast.className = 'notification-toast';
        toast.setAttribute('role', 'alert');

        // Toast content
        const content = document.createElement('div');
        content.className = 'notification-toast-content';

        const title = document.createElement('div');
        title.className = 'notification-toast-title';
        title.textContent = notification.title;

        const message = document.createElement('div');
        message.className = 'notification-toast-message';
        message.textContent = notification.message;

        content.appendChild(title);
        content.appendChild(message);

        // Close button
        const closeBtn = document.createElement('button');
        closeBtn.className = 'notification-toast-close';
        closeBtn.setAttribute('aria-label', 'Dismiss notification');
        closeBtn.innerHTML = '&times;';
        closeBtn.addEventListener('click', function() {
            removeToast(toast);
        });

        toast.appendChild(content);
        toast.appendChild(closeBtn);

        // Make toast clickable if it has a URL
        if (notification.url) {
            toast.style.cursor = 'pointer';
            toast.addEventListener('click', function(e) {
                if (e.target !== closeBtn) {
                    window.location.href = notification.url;
                }
            });
        }

        // Add to container
        container.appendChild(toast);

        // Trigger animation
        requestAnimationFrame(function() {
            toast.classList.add('notification-toast-show');
        });

        // Auto-dismiss after duration
        setTimeout(function() {
            removeToast(toast);
        }, TOAST_DURATION);
    }

    /**
     * Remove a toast with animation
     */
    function removeToast(toast) {
        if (!toast || !toast.parentNode) return;

        toast.classList.remove('notification-toast-show');
        toast.classList.add('notification-toast-hide');

        // Remove from DOM after animation
        setTimeout(function() {
            if (toast.parentNode) {
                toast.parentNode.removeChild(toast);
            }
        }, 300);
    }

    /**
     * Send a ping to keep the connection alive
     */
    function sendPing() {
        if (ws && ws.readyState === WebSocket.OPEN) {
            ws.send('ping');
        }
    }

    // Start ping interval (every 25 seconds, before 30 second timeout)
    setInterval(sendPing, 25000);

    // Handle page visibility changes
    document.addEventListener('visibilitychange', function() {
        if (document.visibilityState === 'visible') {
            // Page became visible, check connection
            if (!ws || ws.readyState !== WebSocket.OPEN) {
                reconnectAttempts = 0; // Reset attempts when user returns
                connect();
            }
        }
    });

    // Clean up on page unload
    window.addEventListener('beforeunload', function() {
        if (ws && ws.readyState === WebSocket.OPEN) {
            ws.close(1000, 'Page unload');
        }
    });

    // Initial connection
    connect();
});
