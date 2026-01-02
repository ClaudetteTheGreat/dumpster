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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_html_escapes_less_than() {
        assert_eq!(sanitize_html("<script>"), "&lt;script&gt;");
    }

    #[test]
    fn test_sanitize_html_escapes_ampersand() {
        assert_eq!(sanitize_html("foo & bar"), "foo &amp; bar");
    }

    #[test]
    fn test_sanitize_html_escapes_quotes() {
        assert_eq!(sanitize_html("say \"hello\""), "say &quot;hello&quot;");
        assert_eq!(sanitize_html("it's"), "it&#x27;s");
    }

    #[test]
    fn test_sanitize_html_escapes_all_special_chars() {
        let input = "<script>alert('xss' & \"bad\")</script>";
        let expected = "&lt;script&gt;alert(&#x27;xss&#x27; &amp; &quot;bad&quot;)&lt;/script&gt;";
        assert_eq!(sanitize_html(input), expected);
    }

    #[test]
    fn test_sanitize_html_preserves_normal_text() {
        assert_eq!(sanitize_html("Hello World"), "Hello World");
        assert_eq!(sanitize_html("Test 123"), "Test 123");
    }

    #[test]
    fn test_close_spoiler_tag() {
        assert_eq!(super::super::Tag::close_spoiler_tag(), "</details>");
    }

    #[test]
    fn test_close_inline_spoiler_tag() {
        assert_eq!(super::super::Tag::close_inline_spoiler_tag(), "</span>");
    }
}
