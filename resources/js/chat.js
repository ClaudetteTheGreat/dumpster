import { MicroModal } from 'micromodal';

document.addEventListener("DOMContentLoaded", function () {
    let scrollEl = document.getElementById('chat-scroller');

    // Only run chat code if we're on the chat page
    if (!scrollEl) {
        return;
    }

    let ws = null;
    let room = null;
    let messageHoverEl = null;
    let userHover = null;
    let lastScrollPos = 0;
    let userActivityData = {};

    function inputAddEventListeners(el) {
        // Keyboard shortcuts for formatting (Ctrl+B, Ctrl+I, Ctrl+U)
        el.addEventListener('keydown', function (event) {
            if (event.ctrlKey || event.metaKey) {
                switch (event.key.toLowerCase()) {
                    case 'b':
                        event.preventDefault();
                        document.execCommand('bold', false, null);
                        return false;
                    case 'i':
                        event.preventDefault();
                        document.execCommand('italic', false, null);
                        return false;
                    case 'u':
                        event.preventDefault();
                        document.execCommand('underline', false, null);
                        return false;
                }
            }
        });

        // Paste handler - preserve simple formatting, strip complex HTML
        el.addEventListener('paste', function (event) {
            event.preventDefault();

            // Try to get HTML content first for rich paste
            let html = event.clipboardData.getData('text/html');
            if (html) {
                // Parse and clean the HTML, keeping only simple formatting
                const temp = document.createElement('div');
                temp.innerHTML = html;

                // Remove all elements except simple formatting
                const allowedTags = ['B', 'STRONG', 'I', 'EM', 'U', 'S', 'STRIKE', 'DEL', 'BR'];
                function cleanNode(node) {
                    const children = Array.from(node.childNodes);
                    for (const child of children) {
                        if (child.nodeType === Node.ELEMENT_NODE) {
                            if (!allowedTags.includes(child.tagName)) {
                                // Replace with its text content
                                const text = document.createTextNode(child.textContent);
                                node.replaceChild(text, child);
                            } else {
                                cleanNode(child);
                            }
                        }
                    }
                }
                cleanNode(temp);

                // Insert the cleaned HTML
                document.execCommand('insertHTML', false, temp.innerHTML);
                return false;
            }

            // Fallback to plain text
            var text = event.clipboardData.getData('text/plain');
            const sel = window.getSelection();
            if (!sel.rangeCount) {
                return false;
            }
            sel.deleteFromDocument();

            let range = sel.getRangeAt(0);
            let newNode = document.createTextNode(text);
            range.insertNode(newNode);
            range.setStart(el, range.endOffset);

            return false;
        });
    }

    function inputFocusEnd(el) {
        setTimeout(function () {
            let range = document.createRange();
            range.setStart(el, el.childElementCount + 1);
            range.setEnd(el, el.childElementCount + 1);
            range.collapse(false);

            let sel = window.getSelection();
            sel.removeAllRanges();
            sel.addRange(range);
            el.focus();
        }, 0);
    }

    function messageAddEventListeners(element) {
        if (Object.keys(element.dataset).indexOf('author') > -1) {
            element.addEventListener('mouseenter', messageMouseEnter);
            element.addEventListener('mouseleave', messageMouseLeave);
        }

        let authorEl = element.querySelector('.author');
        if (authorEl !== null) {
            authorEl.addEventListener('click', usernameClick);
        }

        Array.from(element.querySelectorAll('.username')).forEach(function (usernameEl) {
            usernameEl.addEventListener('click', usernameClick);
            usernameEl.addEventListener('mouseenter', usernameEnter);
            usernameEl.addEventListener('mouseleave', usernameLeave);
        });

        Array.from(element.querySelectorAll('.button')).forEach(function (buttonEl) {
            switch (buttonEl.classList[1]) {
                case 'edit': buttonEl.addEventListener('click', messageButtonEdit); break;
                case 'delete': buttonEl.addEventListener('click', messageButtonDelete); break;
                case 'report': /* buttonEl.addEventListener('click', messageButtonReport); */ break;
                default: console.log("Unable to find use for button.", buttonEl); break;
            }
        });
    }

    function messageButtonDelete() {
        let messageEl = this.closest(".chat-message");
        if (messageEl !== null) {
            let template = document.getElementById('tmp-chat-modal-delete').content.cloneNode(true);
            let modal = template.children[0];
            modal.id = "chat-modal-delete";

            let cloneEl = messageEl.cloneNode(true);
            cloneEl.classList.remove("chat-message--hasParent");
            cloneEl.querySelector('.right-content').remove();

            modal.querySelector('.modal-message').appendChild(cloneEl);
            modal.querySelector('.button.cancel').addEventListener('click', function () {
                window.MicroModal.close(modal.id);
            })
            modal.querySelector('.button.delete').addEventListener('click', function () {
                messageSend(`/delete ${messageEl.dataset.id}`);
                window.MicroModal.close(modal.id);
            })

            document.body.appendChild(modal);

            // https://micromodal.vercel.app/#configuration
            window.MicroModal.show(modal.id, {
                onClose: modal => document.getElementById(modal.id).remove(),
                openClass: 'is-open',
                disableScroll: true,
                disableFocus: false,
                awaitOpenAnimation: false,
                awaitCloseAnimation: false,
                debugMode: false
            });
        }
        else {
            console.log("Error: Cannot find chat message for delete button?");
        }
    }

    function messageButtonEdit() {
        let messageEl = this.closest(".chat-message");
        if (messageEl !== null) {
            messageEdit(messageEl);
        }
        else {
            console.log("Error: Cannot find chat message for delete button?");
        }
    }

    function messageDelete(message) {
        let el = document.getElementById(`chat-message-${message}`);
        let next = el.nextElementSibling;

        el.remove();
        messageSetHasParent(next);

        lastScrollPos = 0;
    }

    function messageEdit(messageEl) {
        messageEditReverse();

        messageEl.classList.add("chat-message--editing");

        let contentEl = messageEl.querySelector('.message');
        messageEl.originalMessage = contentEl.outerHTML;

        let formEl = document.getElementById("new-message-form").cloneNode(true);
        formEl.id = "edit-message-form";

        let inputEl = formEl.querySelector(".chat-input");
        inputEl.id = "edit-message-input";

        let submitEl = formEl.querySelector("button.submit");
        //submitEl.id = "edit-message-input";
        submitEl.remove();

        contentEl.replaceWith(formEl);

        // Convert BBCode to HTML for WYSIWYG editing
        inputEl.innerHTML = bbcodeToHtml(messageEl.rawMessage);
        inputAddEventListeners(inputEl);
        inputEl.addEventListener('keydown', function (event) {
            switch (event.key) {
                case "Escape":
                    event.preventDefault();
                    messageEditReverse();
                    return false;

                case "Enter":
                    if (event.shiftKey) {
                        // Allow Shift+Enter to insert newline
                        return true;
                    }
                    event.preventDefault();

                    messageSend("/edit " + JSON.stringify({
                        id: parseInt(messageEl.dataset.id, 10),
                        message: getInputBBCode(this),
                    }));
                    messageEditReverse();

                    return false;
            }
        });

        // Apparently, .focus() doesn't work on contenteditable=true until one frame after.
        inputFocusEnd(inputEl);
    }

    function messageEditReverse() {
        Array.from(document.querySelectorAll('.chat-message--editing')).forEach(function (el) {
            let contentEl = el.querySelector('.chat-form').outerHTML = el.originalMessage;
            el.classList.remove("chat-message--editing");
            lastScrollPos = 0;
            document.getElementById('new-message-input').focus({ preventScroll: true });
        });
    }

    function messageMouseEnter(event) {
        var author = parseInt(this.dataset.author, 10);

        // Are we already hovering over something?
        if (messageHoverEl !== null) {
            // Is it the same message?
            if (this == messageHoverEl) {
                // We don't need to do anything.
                return true;
            }

            // Is it by the same author?
            if (author === parseInt(messageHoverEl.dataset.author, 10)) {
                // Great, we don't need to do anything.
                //messageHoverEl = $msg;
                //chat.$msgs.children().removeClass(chat.classes.highlightHover);
                //$msg.addClass(chat.classes.highlightHover);
                return true;
            }
        }

        messageHoverEl = this;

        Array.from(document.querySelectorAll('.chat-message--highlightAuthor')).forEach(function (el) {
            el.classList.remove('chat-message--highlightAuthor');
        });

        Array.from(document.querySelectorAll(`.chat-message[data-author='${author}']`)).forEach(function (el) {
            el.classList.add('chat-message--highlightAuthor');
        });
    }

    function messageMouseLeave(event) {
        // We only need to do anything if we're hovering over this message.
        // If we moved between messages, this work is already done.
        if (messageHoverEl !== null && messageHoverEl == this) {
            // We are off of any message, so remove the hovering classes.
            messageHoverEl = null;
            Array.from(document.querySelectorAll('.chat-message--highlightAuthor')).forEach(function (el) {
                el.classList.remove('chat-message--highlightAuthor');
            });
        }
    }

    function messagePush(message, author) {
        if (typeof message === 'string') {
            message = { message: message };
        }

        let id = null;
        let extantEl = null;
        let messagesEl = document.getElementById('chat-messages');
        let template = document.getElementById('tmp-chat-message').content.cloneNode(true);

        template.querySelector('.message').innerHTML = message.message;

        if (author) {
            id = parseInt(message.message_id, 10);
            extantEl = document.getElementById(`chat-message-${id}`);

            template.children[0].rawMessage = message.message_raw;
            template.children[0].id = `chat-message-${id}`;
            template.children[0].dataset.id = id;
            template.children[0].dataset.author = author.id;
            template.children[0].dataset.timestamp = message.message_date;

            // Ignored poster?
            if (APP.user.ignored_users.includes(author.id)) {
                template.children[0].classList.add("chat-message--isIgnored");
            }

            // Add meta details
            let authorEl = template.querySelector('.author');
            authorEl.innerHTML = author.username;
            authorEl.dataset.id = author.id;

            Array.from(template.querySelectorAll('.timestamp')).forEach(function (el) {
                let time = new Date(message.message_date * 1000);
                let hours = time.getHours();
                let minutes = String(time.getMinutes()).padStart(2, '0');

                el.setAttribute('datetime', message.message_date);

                if (el.classList.contains('relative')) {
                    let dayThen = new Date(message.message_date * 1000).setHours(0, 0, 0, 0);
                    let dayNow = new Date().setHours(0, 0, 0, 0);

                    // Same day, only show clock
                    if (dayThen == dayNow) {
                        el.innerHTML = time.toLocaleTimeString();
                    }
                    // Different days, show date too.
                    else {
                        el.innerHTML = time.toLocaleDateString() + " " + time.toLocaleTimeString()
                    }
                }
                else {
                    el.innerHTML = (hours % 12) + ":" + minutes + " " + (hours >= 12 ? "PM" : "AM");
                }
            });

            // Add left-content details
            if (author.avatar_url.length > 0) {
                template.querySelector('.avatar').setAttribute('src', author.avatar_url);
            }
            else {
                template.querySelector('.avatar').remove();
            }

            // Add right-content details
            if (message.author.id != APP.user.id) {
                template.querySelector('.edit').remove();

                if (!APP.user.is_staff) {
                    template.querySelector('.delete').remove();
                }
            }
            template.querySelector('.report').setAttribute('href', `/chat/messages/${message.message_id}/report`);
        }
        else {
            template.children[0].classList.add("chat-message--systemMsg");
            template.querySelector('.meta').remove();
            template.querySelector('.left-content').remove();
            //template.querySelector('.right-content').remove();
        }

        // TODO: FIND SOMETHING BETTER FOR THIS
        // Force set URLs to target new tab.
        Array.from(template.querySelectorAll('.tagUrl')).forEach(function (el) {
            el.target = "_blank";
        });

        // Check tagging.
        if (message.message.includes(`@${APP.user.username}`)) {
            template.children[0].classList.add("chat-message--highlightYou");
        }

        let el = template.children[0];
        messageAddEventListeners(el);

        if (extantEl !== null) {
            extantEl.replaceWith(el);
        }
        else {
            el = messagesEl.appendChild(el);
        }

        messageSetHasParent(el);

        // Prune oldest messages.
        while (messagesEl.children.length > 200) {
            messagesEl.children[0].remove();
            lastScrollPos = 0;
        }

        messagesEl.children[0].classList.remove("chat-message--hasParent");

        // Scroll down.
        scrollToNew();

        return el;
    }

    function messageSend(message) {
        ws.send(message);
    }

    function messageSetHasParent(el) {
        let prev = el.previousElementSibling;

        if (prev !== null) {
            if (prev.dataset.author == el.dataset.author) {
                // Allow to break into new groups if too much time has passed.
                let timeLast = parseInt(prev.dataset.timestamp, 10);
                let timeNext = parseInt(el.dataset.timestamp, 10);
                if (timeNext - timeLast < 30) {
                    el.classList.add("chat-message--hasParent");
                    return true;
                }
            }
        }

        el.classList.remove("chat-message--hasParent");
        return false;
    }

    function messagesReceive(data) {
        let json = null;

        // Try to parse JSON data.
        try {
            json = JSON.parse(data);
        }
        // Not valid JSON, default
        catch (error) {
            messagePush({ message: data }, null);
            return;
        }

        if (json.hasOwnProperty('messages')) {
            json.messages.forEach(message => messagePush(message, message.author));
        }

        if (json.hasOwnProperty('delete')) {
            json.delete.forEach(message => messageDelete(message));
        }

        if (json.hasOwnProperty('users')) {
            Object.entries(json.users).forEach(user => {
                let [key, value] = user;
                userActivity(key, value);
            });
            userActivitySort();
        }
    }

    function messagesDelete() {
        let messagesEl = document.getElementById('chat-messages');
        while (messagesEl.firstChild) {
            messagesEl.removeChild(messagesEl.firstChild);
        }
    }

    function roomJoin(id) {
        if (Number.isInteger(id) && id > 0) {
            scrollEl.classList.remove('ScrollAnchored');
            scrollEl.classList.add('ScrollAnchorConsume');
            messagesDelete();
            userActivityDelete();
            messageSend(`/join ${id}`);
            //document.getElementById("chat-input").focus({ preventScroll: true });
            return true;
        }

        console.log(`Attempted to join a room with an ID of ${room_id}`);
        return false;
    }

    function roomJoinByHash() {
        let room_id = parseInt(window.location.hash.substring(1), 10);

        if (room_id > 0) {
            return roomJoin(room_id);
        }

        return false;
    }

    function scrollerScroll(event) {
        const clampHeight = 64; // margin of error

        // if last scrollTop is lower (greater) than current scroll top,
        // we have scrolled down.
        if (lastScrollPos > this.scrollTop) {
            if (!this.classList.contains("ScrollAnchorConsume")) {
                this.classList.add('ScrollAnchored');
            }
            else {
                this.classList.remove('ScrollAnchorConsume');
            }
        }
        // if we've scrolled down and we are very close to the bottom
        // based on the height of the viewport, lock it in
        else if (this.offsetHeight + this.scrollTop >= this.scrollHeight - clampHeight) {
            this.classList.remove('ScrollAnchored');
        }

        lastScrollPos = this.scrollTop;
    }

    function scrollToNew() {
        if (!scrollEl.classList.contains('ScrollAnchored')) {
            scrollEl.scrollTo(0, scrollEl.scrollHeight);
        }
    }

    function userActivity(id, activity) {
        if (id == 0)
            return;

        let userEl = document.getElementById(`chat-activity-${id}`);

        if (activity !== false) {
            userActivityData[id] = activity;
            userActivityTouch(id);
        }
        else {
            delete userActivityData[id];

            if (userEl) {
                userEl.remove();
            }
        }
    }

    function userActivityDelete() {
        let userEl = document.getElementById(`chat-activity`);
        while (userEl.firstChild) {
            userEl.removeChild(userEl.firstChild);
        }
    }

    function userActivityTouch(id) {
        if (userActivityData[id]) {
            let userEl = document.getElementById(`chat-activity-${id}`);
            userActivityData[id].last_activity = new Date;

            if (userEl) {
                // ???
            }
            else {
                let usersEl = document.getElementById('chat-activity');
                let newEl = document.getElementById('tmp-chat-user').content.cloneNode(true).children[0];

                newEl.id = `chat-activity-${id}`;
                newEl.dataset.username = userActivityData[id].username;
                newEl.last_activity = userActivityData[id].last_activity;

                let avEl = newEl.querySelector('.avatar');
                if (userActivityData[id].avatar_url) {
                    avEl.src = userActivityData[id].avatar_url;
                    avEl.alt = userActivityData[id].username;
                }
                else {
                    avEl.remove();
                }

                let nameEl = newEl.querySelector('.user');
                nameEl.textContent = userActivityData[id].username;

                usersEl.appendChild(newEl);
            }
        }
    }

    function userActivitySort() {
        let usersEl = document.getElementById('chat-activity');
        let activityEls = usersEl.querySelectorAll(".activity");
        let time = (new Date).getTime();

        let sorted = Array.from(activityEls).sort((a, b) => {
            let ar = (a.last_activity.getTime() - time) <= 30000;
            let br = (b.last_activity.getTime() - time) <= 30000;

            if (ar == br) {
                return a.dataset.username.toLowerCase().localeCompare(b.dataset.username.toLowerCase());
            }
            else if (ar && !br) {
                return -1;
            }
            else {
                return 1;
            }
        })

        sorted.forEach(e => usersEl.appendChild(e));
    }

    function usernameClick(event) {
        // TODO: Replace with Dialog like Discord?
        let inputEl = document.getElementById('new-message-input')
        inputEl.textContent += `@${this.textContent}, `;

        inputFocusEnd(inputEl);

        event.preventDefault();
        return false;
    }

    function usernameEnter(event) {
        var id = parseInt(this.dataset.id, 10);

        if (userHover === id) {
            return true;
        }

        userHover = id;

        Array.from(document.querySelectorAll('.chat-message--highlightUser')).forEach(function (el) {
            el.classList.remove('chat-message--highlightUser');
        });
        Array.from(document.querySelectorAll(`[data-author='${id}']`)).forEach(function (el) {
            el.classList.add('chat-message--highlightUser');
        });
    }

    function usernameLeave(event) {
        var id = parseInt(this.dataset.id, 10);

        // Are we hovering over the same message still?
        // This stops unhovering when moving between hover targets.
        if (userHover === id) {
            userHover = null;
            Array.from(document.querySelectorAll('.chat-message--highlightUser')).forEach(function (el) {
                el.classList.remove('chat-message--highlightUser');
            });
        }
    }

    function websocketConnect() {
        // TODO: Make this something practical.
        // fixes cross-domain issues that the forum currently enjoy
        // transform "wss://mysite.us/rust-chat" to "wss://mysite.eu/rust-chat" when on mysite.eu, for instance.
        let sneed = new URL(APP.chat_ws_url);
        sneed.hostname = window.location.hostname;
        sneed.protocol = window.location.protocol == "http:" ? "ws:" : "wss:";

        ws = new WebSocket(sneed.href);
        messagePush("Connecting to SneedChat...");

        ws.addEventListener('close', function (event) {
            messagePush("Connection lost. Please wait - attempting reestablish");
            setTimeout(websocketConnect, 3000);
        });

        ws.addEventListener('error', function (event) {
            console.log(event);
        });

        ws.addEventListener('message', function (event) {
            messagesReceive(event.data);
        });

        ws.addEventListener('open', function (event) {
            if (room === null) {
                if (!roomJoinByHash()) {
                    messagePush("Connected! You may now join a room.");
                }
                else {
                    messagePush("Connected!");
                }
            }
            else {
                messagePush(`Connected to <em>${room.title}</em>!`);
            }
        });
    }


    // Room buttons
    //document.getElementById('chat-rooms').addEventListener('click', function (event) {
    //    let target = event.target;
    //    if (target.classList.contains('chat-room')) {
    //        let room_id = parseInt(target.dataset.id, 10);
    //
    //        if (!isNaN(room_id) && room_id > 0) {
    //            messageSend(`/join ${room_id}`);
    //        }
    //        else {
    //            console.log(`Attempted to join a room with an ID of ${room_id}`);
    //        }
    //    }
    //});

    // Scroll window
    scrollEl.addEventListener('scroll', scrollerScroll);
    //scrollEl.classList.add('ScrollLocked');
    setInterval(scrollToNew, 32);

    // Form
    document.getElementById('new-message-input').addEventListener('keydown', function (event) {
        switch (event.key) {
            case "Enter":
                if (event.shiftKey) {
                    // Allow Shift+Enter to insert newline
                    return true;
                }
                event.preventDefault();

                messageSend(getInputBBCode(this));
                this.innerHTML = "";

                return false;

            case "ArrowUp":
                if (!this.innerHTML) {
                    event.preventDefault();

                    let messageEls = document.getElementById('chat-messages').querySelectorAll(`.chat-message[data-author='${APP.user.id}']`);
                    if (messageEls.length > 0) {
                        let messageEl = messageEls[messageEls.length - 1];
                        messageEdit(messageEl);
                    }

                    return false;
                }
        }
    });
    inputAddEventListeners(document.getElementById('new-message-input'));

    // Chat toolbar functionality
    initChatToolbar();

    // Blur spoiler click handler
    initBlurSpoilers();

    // Color picker
    initColorPicker();

    function initBlurSpoilers() {
        // Use event delegation on the messages container
        const messagesEl = document.getElementById('chat-messages');
        if (!messagesEl) return;

        messagesEl.addEventListener('click', function(event) {
            const spoiler = event.target.closest('.blur-spoiler');
            if (spoiler) {
                spoiler.classList.toggle('revealed');
            }
        });
    }

    function initColorPicker() {
        const picker = document.querySelector('.chat-color-picker');
        if (!picker) return;

        const btn = picker.querySelector('.chat-color-btn');
        const dropdown = picker.querySelector('.chat-color-dropdown');
        const swatches = picker.querySelectorAll('.chat-color-swatch');
        const colorInput = picker.querySelector('.chat-color-input');
        const applyBtn = picker.querySelector('.chat-color-apply');
        const inputEl = document.getElementById('new-message-input');

        // Toggle dropdown
        btn.addEventListener('click', function(event) {
            event.preventDefault();
            event.stopPropagation();
            picker.classList.toggle('open');
        });

        // Close dropdown when clicking outside
        document.addEventListener('click', function(event) {
            if (!picker.contains(event.target)) {
                picker.classList.remove('open');
            }
        });

        // Apply color from preset swatches
        swatches.forEach(function(swatch) {
            swatch.addEventListener('click', function(event) {
                event.preventDefault();
                const color = this.dataset.color;
                applyColor(color);
                picker.classList.remove('open');
            });
        });

        // Apply custom color
        applyBtn.addEventListener('click', function(event) {
            event.preventDefault();
            applyColor(colorInput.value);
            picker.classList.remove('open');
        });

        function applyColor(color) {
            inputEl.focus();
            document.execCommand('foreColor', false, color);
        }
    }

    function initChatToolbar() {
        const toolbar = document.querySelector('.chat-toolbar');
        if (!toolbar) return;

        const inputEl = document.getElementById('new-message-input');

        toolbar.addEventListener('click', function(event) {
            const btn = event.target.closest('.chat-toolbar-btn');
            if (!btn) return;

            event.preventDefault();
            const tag = btn.dataset.bbcode;
            if (!tag) return;

            // Ensure focus is on input before applying formatting
            inputEl.focus();

            // Use execCommand for inline formatting marks
            const inlineMarks = {
                'b': 'bold',
                'i': 'italic',
                'u': 'underline',
                's': 'strikeThrough'
            };

            if (inlineMarks[tag]) {
                document.execCommand(inlineMarks[tag], false, null);
                return;
            }

            // Handle special tags that need prompts
            insertChatBBCode(inputEl, tag);
        });
    }

    function insertChatBBCode(el, tag) {
        const sel = window.getSelection();
        let selectedText = '';

        // Get selected text if selection is within the input
        if (sel.rangeCount > 0) {
            const range = sel.getRangeAt(0);
            if (el.contains(range.commonAncestorContainer)) {
                selectedText = range.toString();
            }
        }

        // Handle special tags that need prompts
        if (tag === 'url') {
            const url = prompt('Enter URL:', selectedText.startsWith('http') ? selectedText : 'https://');
            if (!url) return;
            if (selectedText && !selectedText.startsWith('http')) {
                insertTextAtCursor(el, `[url=${url}]${selectedText}[/url]`);
            } else {
                insertTextAtCursor(el, `[url]${url}[/url]`);
            }
            return;
        }

        if (tag === 'img') {
            const url = prompt('Enter image URL:', selectedText.startsWith('http') ? selectedText : 'https://');
            if (!url) return;
            insertTextAtCursor(el, `[img]${url}[/img]`);
            return;
        }

        // Standard wrapping tags (spoiler, code)
        const openTag = `[${tag}]`;
        const closeTag = `[/${tag}]`;
        insertTextAtCursor(el, openTag + selectedText + closeTag);
    }

    function insertTextAtCursor(el, text) {
        el.focus();
        const sel = window.getSelection();

        if (sel.rangeCount > 0) {
            const range = sel.getRangeAt(0);
            if (el.contains(range.commonAncestorContainer)) {
                range.deleteContents();
                const textNode = document.createTextNode(text);
                range.insertNode(textNode);
                range.setStartAfter(textNode);
                range.setEndAfter(textNode);
                sel.removeAllRanges();
                sel.addRange(range);
                return;
            }
        }

        // Fallback: append to end
        el.appendChild(document.createTextNode(text));
    }

    /**
     * Convert HTML from contenteditable to BBCode
     * Handles inline formatting tags: b, i, u, s, strong, em, font color, span color
     */
    function htmlToBBCode(html) {
        // Create a temporary element to parse the HTML
        const temp = document.createElement('div');
        temp.innerHTML = html;

        function processNode(node) {
            if (node.nodeType === Node.TEXT_NODE) {
                return node.textContent;
            }

            if (node.nodeType !== Node.ELEMENT_NODE) {
                return '';
            }

            // Get the content of child nodes first
            let content = '';
            for (const child of node.childNodes) {
                content += processNode(child);
            }

            // Map HTML tags to BBCode
            const tagName = node.tagName.toLowerCase();
            switch (tagName) {
                case 'b':
                case 'strong':
                    return `[b]${content}[/b]`;
                case 'i':
                case 'em':
                    return `[i]${content}[/i]`;
                case 'u':
                    return `[u]${content}[/u]`;
                case 's':
                case 'strike':
                case 'del':
                    return `[s]${content}[/s]`;
                case 'font':
                    // Handle <font color="..."> from execCommand
                    const fontColor = node.getAttribute('color');
                    if (fontColor) {
                        return `[color=${fontColor}]${content}[/color]`;
                    }
                    return content;
                case 'span':
                    // Handle <span style="color: ..."> from some browsers
                    const style = node.getAttribute('style');
                    if (style) {
                        const colorMatch = style.match(/color:\s*([^;]+)/i);
                        if (colorMatch) {
                            let color = colorMatch[1].trim();
                            // Convert rgb() to hex if needed
                            if (color.startsWith('rgb')) {
                                color = rgbToHex(color);
                            }
                            return `[color=${color}]${content}[/color]`;
                        }
                    }
                    return content;
                case 'br':
                    return '\n';
                case 'div':
                case 'p':
                    // Block elements add newlines
                    return content + '\n';
                default:
                    return content;
            }
        }

        let result = '';
        for (const child of temp.childNodes) {
            result += processNode(child);
        }

        // Clean up extra newlines
        return result.replace(/\n+$/, '').replace(/\n{3,}/g, '\n\n');
    }

    /**
     * Convert RGB color string to hex
     */
    function rgbToHex(rgb) {
        const match = rgb.match(/rgb\((\d+),\s*(\d+),\s*(\d+)\)/);
        if (!match) return rgb;
        const r = parseInt(match[1]).toString(16).padStart(2, '0');
        const g = parseInt(match[2]).toString(16).padStart(2, '0');
        const b = parseInt(match[3]).toString(16).padStart(2, '0');
        return `#${r}${g}${b}`;
    }

    /**
     * Convert BBCode to HTML for editing
     * Handles basic inline tags: b, i, u, s, color
     */
    function bbcodeToHtml(bbcode) {
        if (!bbcode) return '';

        let html = bbcode
            // Escape HTML first
            .replace(/&/g, '&amp;')
            .replace(/</g, '&lt;')
            .replace(/>/g, '&gt;')
            // Convert BBCode to HTML
            .replace(/\[b\]([\s\S]*?)\[\/b\]/gi, '<b>$1</b>')
            .replace(/\[i\]([\s\S]*?)\[\/i\]/gi, '<i>$1</i>')
            .replace(/\[u\]([\s\S]*?)\[\/u\]/gi, '<u>$1</u>')
            .replace(/\[s\]([\s\S]*?)\[\/s\]/gi, '<s>$1</s>')
            .replace(/\[color=([^\]]+)\]([\s\S]*?)\[\/color\]/gi, '<font color="$1">$2</font>')
            // Convert newlines to <br> for display
            .replace(/\n/g, '<br>');

        return html;
    }

    /**
     * Get BBCode content from a chat input element
     */
    function getInputBBCode(el) {
        return htmlToBBCode(el.innerHTML);
    }

    document.getElementById('new-message-submit').addEventListener('click', function (event) {
        event.preventDefault();
        let input = document.getElementById('new-message-input');

        messageSend(getInputBBCode(input));
        input.innerHTML = "";

        input.focus({ preventScroll: true });
        return false;
    });

    // Safely terminate websocket so server knows we're disconnecting.
    window.addEventListener('beforeunload', function () {
        if (ws.readyState == WebSocket.OPEN) {
            websocket.onclose = function () { };
            websocket.close(1000, "Bye!");
        }
    });

    window.addEventListener('hashchange', roomJoinByHash, false);

    window.addEventListener('keydown', function (event) {
        switch (event.key) {
            case "Escape": messageEditReverse(); break;
        }
    });

    window.addEventListener('resize', function (event) {
        if (!scrollEl.classList.contains("ScrollAnchor")) {
            scrollEl.classList.add("ScrollAnchorConsume");
        }
        scrollToNew();
    });

    websocketConnect();
});
