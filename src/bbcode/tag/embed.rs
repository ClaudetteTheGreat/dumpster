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

/// Media type for auto-detection
enum MediaType {
    YouTube(String), // video ID
    Vimeo(String),   // video ID
    Video(String),   // direct URL
    Audio(String),   // direct URL
    Unknown,
}

/// Extract YouTube video ID from various URL formats
fn extract_youtube_id(url: &Url) -> Option<String> {
    let host = url.host_str()?;

    // youtube.com/watch?v=ID or youtube.com/embed/ID
    if host == "youtube.com" || host == "www.youtube.com" {
        if url.path() == "/watch" {
            return url
                .query_pairs()
                .find(|(k, _)| k == "v")
                .map(|(_, v)| v.to_string());
        }
        if let Some(id) = url.path().strip_prefix("/embed/") {
            return Some(id.split('?').next().unwrap_or(id).to_string());
        }
    }

    // youtu.be/ID (short URLs)
    if host == "youtu.be" {
        return url.path().strip_prefix('/').map(|s| s.to_string());
    }

    None
}

/// Extract Vimeo video ID from URL
fn extract_vimeo_id(url: &Url) -> Option<String> {
    let host = url.host_str()?;

    if host == "vimeo.com" || host == "www.vimeo.com" || host == "player.vimeo.com" {
        // vimeo.com/123456789 or player.vimeo.com/video/123456789
        let path = url.path().strip_prefix("/video/").unwrap_or(url.path());
        let id = path.strip_prefix('/').unwrap_or(path);

        // Validate that it's numeric
        if id.chars().all(|c| c.is_ascii_digit()) && !id.is_empty() {
            return Some(id.to_string());
        }
    }

    None
}

/// Check if URL points to a video file
fn is_video_url(url: &Url) -> bool {
    let path = url.path().to_lowercase();
    path.ends_with(".mp4")
        || path.ends_with(".webm")
        || path.ends_with(".ogg")
        || path.ends_with(".ogv")
}

/// Check if URL points to an audio file
fn is_audio_url(url: &Url) -> bool {
    let path = url.path().to_lowercase();
    path.ends_with(".mp3")
        || path.ends_with(".ogg")
        || path.ends_with(".oga")
        || path.ends_with(".wav")
        || path.ends_with(".flac")
        || path.ends_with(".m4a")
}

/// Detect media type from URL
fn detect_media_type(url: &Url) -> MediaType {
    if let Some(id) = extract_youtube_id(url) {
        return MediaType::YouTube(id);
    }
    if let Some(id) = extract_vimeo_id(url) {
        return MediaType::Vimeo(id);
    }
    if is_video_url(url) {
        return MediaType::Video(url.to_string());
    }
    if is_audio_url(url) {
        return MediaType::Audio(url.to_string());
    }
    MediaType::Unknown
}

impl super::Tag {
    // [video]url[/video] - embeds video player
    pub fn open_video_tag(_: RefMut<Element>) -> String {
        String::new()
    }

    pub fn fill_video_tag(mut el: RefMut<Element>, contents: String) -> String {
        if let Ok(url) = Url::parse(&contents) {
            match url.scheme() {
                "http" | "https" => {
                    el.clear_contents();

                    // Check for YouTube or Vimeo first
                    if let Some(id) = extract_youtube_id(&url) {
                        return format!(
                            "<div class=\"video-embed video-embed--youtube\">\
                             <iframe src=\"https://www.youtube-nocookie.com/embed/{}\" \
                             frameborder=\"0\" allowfullscreen loading=\"lazy\" \
                             allow=\"accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture\">\
                             </iframe></div>",
                            id
                        );
                    }

                    if let Some(id) = extract_vimeo_id(&url) {
                        return format!(
                            "<div class=\"video-embed video-embed--vimeo\">\
                             <iframe src=\"https://player.vimeo.com/video/{}\" \
                             frameborder=\"0\" allowfullscreen loading=\"lazy\">\
                             </iframe></div>",
                            id
                        );
                    }

                    // Direct video file
                    if is_video_url(&url) {
                        return format!(
                            "<video class=\"video-embed video-embed--direct\" controls preload=\"metadata\">\
                             <source src=\"{}\" />\
                             Your browser does not support the video tag.\
                             </video>",
                            url.as_str()
                        );
                    }

                    // Unknown video URL - try to embed anyway
                    return format!(
                        "<video class=\"video-embed\" controls preload=\"metadata\">\
                         <source src=\"{}\" />\
                         Your browser does not support the video tag.\
                         </video>",
                        url.as_str()
                    );
                }
                _ => {}
            }
        }

        el.set_broken();
        contents
    }

    // [audio]url[/audio] - embeds audio player
    pub fn open_audio_tag(_: RefMut<Element>) -> String {
        String::new()
    }

    pub fn fill_audio_tag(mut el: RefMut<Element>, contents: String) -> String {
        if let Ok(url) = Url::parse(&contents) {
            match url.scheme() {
                "http" | "https" => {
                    el.clear_contents();
                    return format!(
                        "<audio class=\"audio-embed\" controls preload=\"metadata\">\
                         <source src=\"{}\" />\
                         Your browser does not support the audio tag.\
                         </audio>",
                        url.as_str()
                    );
                }
                _ => {}
            }
        }

        el.set_broken();
        contents
    }

    // [youtube]videoId[/youtube] - YouTube shorthand
    pub fn open_youtube_tag(_: RefMut<Element>) -> String {
        String::new()
    }

    pub fn fill_youtube_tag(mut el: RefMut<Element>, contents: String) -> String {
        let id = contents.trim();

        // Accept either just the ID or a full URL
        let video_id = if id.starts_with("http://") || id.starts_with("https://") {
            if let Ok(url) = Url::parse(id) {
                extract_youtube_id(&url)
            } else {
                None
            }
        } else {
            // Validate ID format (alphanumeric, underscores, hyphens, typically 11 chars)
            if id
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
                && !id.is_empty()
            {
                Some(id.to_string())
            } else {
                None
            }
        };

        if let Some(id) = video_id {
            el.clear_contents();
            return format!(
                "<div class=\"video-embed video-embed--youtube\">\
                 <iframe src=\"https://www.youtube-nocookie.com/embed/{}\" \
                 frameborder=\"0\" allowfullscreen loading=\"lazy\" \
                 allow=\"accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture\">\
                 </iframe></div>",
                id
            );
        }

        el.set_broken();
        contents
    }

    // [media]url[/media] - auto-detect media type
    pub fn open_media_tag(_: RefMut<Element>) -> String {
        String::new()
    }

    pub fn fill_media_tag(mut el: RefMut<Element>, contents: String) -> String {
        if let Ok(url) = Url::parse(&contents) {
            match url.scheme() {
                "http" | "https" => {
                    el.clear_contents();

                    match detect_media_type(&url) {
                        MediaType::YouTube(id) => {
                            return format!(
                                "<div class=\"video-embed video-embed--youtube\">\
                                 <iframe src=\"https://www.youtube-nocookie.com/embed/{}\" \
                                 frameborder=\"0\" allowfullscreen loading=\"lazy\" \
                                 allow=\"accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture\">\
                                 </iframe></div>",
                                id
                            );
                        }
                        MediaType::Vimeo(id) => {
                            return format!(
                                "<div class=\"video-embed video-embed--vimeo\">\
                                 <iframe src=\"https://player.vimeo.com/video/{}\" \
                                 frameborder=\"0\" allowfullscreen loading=\"lazy\">\
                                 </iframe></div>",
                                id
                            );
                        }
                        MediaType::Video(src) => {
                            return format!(
                                "<video class=\"video-embed video-embed--direct\" controls preload=\"metadata\">\
                                 <source src=\"{}\" />\
                                 Your browser does not support the video tag.\
                                 </video>",
                                src
                            );
                        }
                        MediaType::Audio(src) => {
                            return format!(
                                "<audio class=\"audio-embed\" controls preload=\"metadata\">\
                                 <source src=\"{}\" />\
                                 Your browser does not support the audio tag.\
                                 </audio>",
                                src
                            );
                        }
                        MediaType::Unknown => {
                            // Can't determine type, fall through to broken
                        }
                    }
                }
                _ => {}
            }
        }

        el.set_broken();
        contents
    }
}
