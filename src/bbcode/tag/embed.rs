use super::Element;
use std::cell::RefMut;
use url::Url;

impl super::Tag {
    pub fn open_img_tag(_: RefMut<Element>) -> String {
        String::new()
    }

    pub fn fill_img_tag(mut el: RefMut<Element>, contents: String) -> String {
        // Our URL comes from inside the tag.
        if let Ok(url) = Url::parse(&contents) {
            match url.scheme() {
                "http" | "https" => {
                    el.clear_contents();

                    // Check for dimension argument: [img=100x100]url[/img]
                    let dimension_attr = if let Some(arg) = el.get_argument() {
                        let dims = parse_image_dimensions(arg);
                        if dims.is_empty() {
                            String::new()
                        } else {
                            format!(" {}", dims)
                        }
                    } else {
                        String::new()
                    };

                    return format!("<img src=\"{}\"{} />", url.as_str(), dimension_attr);
                }
                _ => {}
            }
        }

        el.set_broken();
        contents
    }

    pub fn open_url_tag(el: RefMut<Element>) -> String {
        if el.is_broken() {
            el.to_open_str()
        } else {
            String::new()
        }
    }

    pub fn fill_url_tag(mut el: RefMut<Element>, contents: String) -> String {
        let mut url: Option<Url> = None;

        if let Some(arg) = el.get_argument() {
            url = match url_arg(arg).transpose() {
                Ok(url) => url,
                Err(_) => {
                    el.set_broken();
                    return contents;
                }
            }
            // TODO: Check for unfurl="true/false"
        }

        if url.is_none() {
            if let Ok(curl) = Url::parse(&contents) {
                url = Some(curl)
            }
        }

        match url {
            Some(url) => format!(
                "<a class=\"bbCode tagUrl\" ref=\"nofollow\" href=\"{}\">{}",
                url.as_str(),
                contents
            ),
            // If we have no content, we are broken.
            None => {
                el.set_broken();
                contents
            }
        }
    }
}

fn url_arg(input: &str) -> Option<Result<Url, &str>> {
    let input = input.strip_prefix('=')?;

    match Url::parse(input) {
        Ok(url) => Some(match url.scheme() {
            "https" => Ok(url),
            "http" => Ok(url),
            _ => Err("Unsupported scheme"),
        }),
        Err(_) => None,
    }
}

/// Parse image dimensions from BBCode argument like "=100x100" or "=100"
/// Returns HTML width/height attributes or empty string if invalid
fn parse_image_dimensions(arg: &str) -> String {
    let input = arg.strip_prefix('=').unwrap_or(arg);

    // Try to parse "WIDTHxHEIGHT" format
    if let Some((width_str, height_str)) = input.split_once('x') {
        if let (Ok(width), Ok(height)) = (width_str.parse::<u32>(), height_str.parse::<u32>()) {
            // Validate dimensions (prevent abuse)
            if width > 0 && width <= 2000 && height > 0 && height <= 2000 {
                return format!("width=\"{}\" height=\"{}\"", width, height);
            }
        }
    }
    // Try to parse just width (maintain aspect ratio)
    else if let Ok(width) = input.parse::<u32>() {
        if width > 0 && width <= 2000 {
            return format!("width=\"{}\"", width);
        }
    }

    // Invalid format or out of range
    String::new()
}
