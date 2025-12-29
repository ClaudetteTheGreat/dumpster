use super::Element;
use std::cell::RefMut;

impl super::Tag {
    /// Opens a quote tag. Returns <blockquote> with optional attribution and link
    /// Format: [quote=username] or [quote=username;thread_id;post_id]
    pub fn open_quote_tag(el: RefMut<Element>) -> String {
        if let Some(arg) = el.get_argument() {
            // Strip leading = if present
            let attribution = arg.strip_prefix('=').unwrap_or(arg).trim();
            if !attribution.is_empty() {
                // Parse format: username or username;thread_id;post_id
                let parts: Vec<&str> = attribution.split(';').collect();
                let username = sanitize_html(parts[0].trim());

                // Check if we have thread_id and post_id for linking
                if parts.len() >= 3 {
                    let thread_id = parts[1].trim();
                    let post_id = parts[2].trim();

                    // Validate IDs are numeric
                    if thread_id.chars().all(|c| c.is_ascii_digit())
                        && post_id.chars().all(|c| c.is_ascii_digit())
                    {
                        let link = format!("/threads/{}/post-{}", thread_id, post_id);
                        return format!(
                            "<blockquote class=\"bbCode tagQuote\" data-author=\"{}\" data-thread=\"{}\" data-post=\"{}\">\
                            <a href=\"{}\" class=\"quote-link\" title=\"Go to original post\">↑</a>\
                            <div class=\"attribution\">{} said:</div>\
                            <div class=\"quoted\">",
                            username, thread_id, post_id, link, username
                        );
                    }
                }

                // No valid link, just show attribution
                return format!(
                    "<blockquote class=\"bbCode tagQuote\" data-author=\"{}\">\
                    <div class=\"attribution\">{} said:</div>\
                    <div class=\"quoted\">",
                    username, username
                );
            }
        }

        String::from("<blockquote class=\"bbCode tagQuote\"><div class=\"quoted\">")
    }

    pub fn close_quote_tag(_el: RefMut<Element>) -> String {
        String::from("</div></blockquote>")
    }
}

/// Sanitizes a string for HTML to prevent XSS
fn sanitize_html(input: &str) -> String {
    let len = input.len();
    let mut output: Vec<u8> = Vec::with_capacity(len * 4);

    for c in input.bytes() {
        match c {
            b'<' => output.extend_from_slice(b"&lt;"),
            b'>' => output.extend_from_slice(b"&gt;"),
            b'&' => output.extend_from_slice(b"&amp;"),
            b'\"' => output.extend_from_slice(b"&quot;"),
            b'\'' => output.extend_from_slice(b"&#x27;"),
            _ => output.push(c),
        }
    }

    unsafe { String::from_utf8_unchecked(output) }
}
