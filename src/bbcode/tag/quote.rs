use super::Element;
use std::cell::RefMut;

impl super::Tag {
    /// Opens a quote tag. Returns <blockquote> with optional attribution
    pub fn open_quote_tag(el: RefMut<Element>) -> String {
        if let Some(arg) = el.get_argument() {
            // Strip leading = if present
            let attribution = arg.strip_prefix('=').unwrap_or(arg).trim();
            if !attribution.is_empty() {
                let sanitized = sanitize_html(attribution);
                return format!(
                    "<blockquote class=\"bbCode tagQuote\" data-author=\"{}\"><div class=\"attribution\">{} said:</div><div class=\"quoted\">",
                    sanitized, sanitized
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
