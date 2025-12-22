use super::Element;
use std::cell::RefMut;

impl super::Tag {
    /// Opens a list tag. Returns either <ul> for unordered or <ol> for ordered lists.
    /// Supports [list], [list=1], and [list=a] syntax.
    pub fn open_list_tag(el: RefMut<Element>) -> String {
        if let Some(arg) = el.get_argument() {
            // Parse list type from argument
            let list_type = arg.strip_prefix('=').unwrap_or("");

            match list_type {
                "1" => String::from("<ol type=\"1\">"),
                "a" | "A" => String::from("<ol type=\"a\">"),
                _ => {
                    // Invalid list type, render as broken tag
                    Self::open_broken_tag(el)
                }
            }
        } else {
            // No argument means unordered list
            String::from("<ul>")
        }
    }

    /// Closes a list tag. Returns either </ul> or </ol> based on list type.
    pub fn close_list_tag(el: RefMut<Element>) -> String {
        if let Some(arg) = el.get_argument() {
            let list_type = arg.strip_prefix('=').unwrap_or("");

            match list_type {
                "1" | "a" | "A" => String::from("</ol>"),
                _ => String::new(), // Broken tags don't close
            }
        } else {
            String::from("</ul>")
        }
    }

    /// Opens a list item tag. Returns <li>
    pub fn open_list_item_tag() -> String {
        String::from("<li>")
    }

    /// Closes a list item tag. Returns </li>
    pub fn close_list_item_tag() -> String {
        String::from("</li>")
    }
}
