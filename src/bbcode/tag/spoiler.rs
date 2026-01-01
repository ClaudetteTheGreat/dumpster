use super::Element;
use std::cell::RefMut;

impl super::Tag {
    /// Opens a spoiler tag. Returns <details><summary>...</summary>
    pub fn open_spoiler_tag(el: RefMut<Element>) -> String {
        let title = if let Some(arg) = el.get_argument() {
            // Strip leading = if present
            let title = arg.strip_prefix('=').unwrap_or(arg).trim();
            if !title.is_empty() {
                sanitize_html(title)
            } else {
                String::from("Spoiler")
            }
        } else {
            String::from("Spoiler")
        };

        format!("<details><summary>{}</summary>", title)
    }

    pub fn close_spoiler_tag() -> String {
        String::from("</details>")
    }

    /// Opens an inline spoiler tag (blur-based). Returns <span class="blur-spoiler">
    pub fn open_inline_spoiler_tag(el: RefMut<Element>) -> String {
        let title = if let Some(arg) = el.get_argument() {
            let title = arg.strip_prefix('=').unwrap_or(arg).trim();
            if !title.is_empty() {
                sanitize_html(title)
            } else {
                String::from("Spoiler")
            }
        } else {
            String::from("Spoiler")
        };

        format!("<span class=\"blur-spoiler\" data-spoiler-title=\"{}\">", title)
    }

    pub fn close_inline_spoiler_tag() -> String {
        String::from("</span>")
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
