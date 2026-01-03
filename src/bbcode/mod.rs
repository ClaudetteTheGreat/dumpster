extern crate linkify;

mod constructor;
mod element;
mod parser;
mod smilie;
mod tag;
mod token;
mod tokenize;

use once_cell::sync::Lazy;
use regex::Regex;

pub use constructor::Constructor;
pub use element::{Element, ElementDisplay};
pub use parser::Parser;
pub use smilie::Smilies;
pub use tag::Tag;
pub use token::Token;
pub use tokenize::tokenize;

/// Regex for matching @mentions
/// Matches @ preceded by start of string, whitespace, or >
static MENTION_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(^|[\s>])@([a-zA-Z0-9_-]+)").unwrap());

/// Convert @mentions to clickable links
/// Skips mentions inside <a>, <pre>, and <code> tags
fn linkify_mentions(html: &str) -> String {
    // Split by tags to avoid processing inside certain elements
    let mut result = String::with_capacity(html.len());
    let mut last_end = 0;
    let mut skip_depth: usize = 0;

    // Simple tag-aware processing
    let bytes = html.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        if bytes[i] == b'<' {
            // Found a tag, check what it is
            let tag_start = i;
            i += 1;

            // Check for closing tag
            let is_closing = i < len && bytes[i] == b'/';
            if is_closing {
                i += 1;
            }

            // Extract tag name
            let name_start = i;
            while i < len && bytes[i] != b'>' && bytes[i] != b' ' && bytes[i] != b'/' {
                i += 1;
            }
            let tag_name = &html[name_start..i].to_lowercase();

            // Skip to end of tag
            while i < len && bytes[i] != b'>' {
                i += 1;
            }
            if i < len {
                i += 1; // past >
            }

            // Check if this is a skip tag (a, pre, code)
            let is_skip_tag = tag_name == "a" || tag_name == "pre" || tag_name == "code";

            // Process text before this tag (using old skip_depth)
            let text_before = &html[last_end..tag_start];
            if !text_before.is_empty() {
                if skip_depth == 0 {
                    result.push_str(&process_mentions(text_before));
                } else {
                    result.push_str(text_before);
                }
            }

            // Update skip_depth for after this tag
            if is_skip_tag {
                if is_closing {
                    skip_depth = skip_depth.saturating_sub(1);
                } else if !html[tag_start..i].ends_with("/>") {
                    skip_depth += 1;
                }
            }

            // Add the tag itself
            result.push_str(&html[tag_start..i]);
            last_end = i;
        } else {
            i += 1;
        }
    }

    // Process remaining text
    let remaining = &html[last_end..];
    if !remaining.is_empty() {
        if skip_depth == 0 {
            result.push_str(&process_mentions(remaining));
        } else {
            result.push_str(remaining);
        }
    }

    result
}

/// Process @mentions in a text segment (no HTML tags)
fn process_mentions(text: &str) -> String {
    MENTION_REGEX
        .replace_all(text, |caps: &regex::Captures| {
            let prefix = &caps[1]; // Whitespace, > or empty string
            let username = &caps[2];

            format!(
                "{}<a class=\"mention\" href=\"/members/@{}\">@{}</a>",
                prefix, username, username
            )
        })
        .to_string()
}

/// Generates a string of HTML from an &str of BbCode.
#[no_mangle]
pub fn parse(input: &str) -> String {
    let tokens: Vec<Token> = tokenize(input).expect("Failed to unwrap tokens.").1;

    //println!("TOKENS: {:?}", tokens);

    let mut parser = Parser::new();
    let ast = parser.parse(&tokens);

    //for node in ast.descendants() {
    //    println!("{:?}", node);
    //}

    let constructor = Constructor::new();
    let html = constructor.build(ast);

    // Post-process to linkify @mentions
    linkify_mentions(&html)
}

#[cfg(test)]
mod tests {
    #[test]
    fn img() {
        use super::parse;

        assert_eq!(
            "<img src=\"https://zombo.com/images/zombocom.png\" />",
            parse("[img]https://zombo.com/images/zombocom.png[/img]")
        );
        assert_eq!(
            "<img src=\"https://zombo.com/images/zombocom.png\" />",
            parse("[img]https://zombo.com/images/zombocom.png")
        );
        assert_eq!("[img][/img]", parse("[img][/img]"));
        assert_eq!("[img]", parse("[img]"));
        assert_eq!("[img]not a link[/img]", parse("[img]not a link[/img]"));
        assert_eq!("[img]not a link", parse("[img]not a link"));

        // Relative URLs (for local content)
        assert_eq!(
            "<img src=\"/content/abc123/image.png\" />",
            parse("[img]/content/abc123/image.png[/img]")
        );
        // Path traversal should be rejected
        assert_eq!(
            "[img]/../etc/passwd[/img]",
            parse("[img]/../etc/passwd[/img]")
        );
    }

    #[test]
    fn inline_tags() {
        use super::parse;

        assert_eq!("<b>Test</b>", parse("[b]Test[/b]"));
        assert_eq!("<b>Test</b>", parse("[B]Test[/B]"));
        assert_eq!("<b>Test</b>", parse("[B]Test[/b]"));
        assert_eq!("<i>Test</i>", parse("[i]Test[/i]"));
        assert_eq!("<u>Test</u>", parse("[u]Test[/u]"));
        assert_eq!("<s>Test</s>", parse("[s]Test[/s]"));

        assert_eq!("<b><i>Test</i></b>", parse("[b][i]Test[/i][/b]"));
        assert_eq!("<b><i>Test</i></b>", parse("[b][i]Test[/i]"));
        assert_eq!("<b><i>Test</i></b>", parse("[b][i]Test[/b]"));
        assert_eq!("<b><i>Test</i></b>", parse("[b][i]Test"));
        assert_eq!("<b><i>Test</i></b>", parse("[B][i]Test"));

        const GOOD_COLORS: &[&str] = &["red", "#ff0000"];
        const BAD_COLORS: &[&str] = &["RED", "ff0000", "sneed", ""];

        for good in GOOD_COLORS {
            assert_eq!(
                format!(
                    "<span class=\"bbCode tagColor\" style=\"color: {}\">Hello!</span>",
                    good
                ),
                parse(&format!("[color={}]Hello![/color]", good))
            );
        }

        for bad in BAD_COLORS {
            assert_eq!(
                format!("[color={}]Hello![/color]", bad),
                parse(&format!("[color={}]Hello![/color]", bad))
            );
        }
    }

    #[test]
    fn size_and_font() {
        use super::parse;

        // Valid sizes (8-36px)
        assert_eq!(
            "<span class=\"bbCode tagSize\" style=\"font-size: 16px;\">Normal text</span>",
            parse("[size=16]Normal text[/size]")
        );
        assert_eq!(
            "<span class=\"bbCode tagSize\" style=\"font-size: 8px;\">Small text</span>",
            parse("[size=8]Small text[/size]")
        );
        assert_eq!(
            "<span class=\"bbCode tagSize\" style=\"font-size: 36px;\">Large text</span>",
            parse("[size=36]Large text[/size]")
        );

        // Invalid sizes (out of range or non-numeric)
        assert_eq!(
            "[size=50]Too large[/size]",
            parse("[size=50]Too large[/size]")
        );
        assert_eq!(
            "[size=5]Too small[/size]",
            parse("[size=5]Too small[/size]")
        );
        assert_eq!(
            "[size=abc]Invalid[/size]",
            parse("[size=abc]Invalid[/size]")
        );

        // Valid fonts
        assert_eq!(
            "<span class=\"bbCode tagFont\" style=\"font-family: arial;\">Arial text</span>",
            parse("[font=arial]Arial text[/font]")
        );
        assert_eq!(
            "<span class=\"bbCode tagFont\" style=\"font-family: verdana;\">Verdana text</span>",
            parse("[font=verdana]Verdana text[/font]")
        );

        // Invalid fonts (not in whitelist)
        assert_eq!(
            "[font=malicious]Blocked[/font]",
            parse("[font=malicious]Blocked[/font]")
        );

        // Nested formatting
        assert_eq!(
            "<span class=\"bbCode tagSize\" style=\"font-size: 20px;\"><b>Bold and sized</b></span>",
            parse("[size=20][b]Bold and sized[/b][/size]")
        );
        assert_eq!(
            "<span class=\"bbCode tagFont\" style=\"font-family: courier;\"><i>Italic courier</i></span>",
            parse("[font=courier][i]Italic courier[/i][/font]")
        );
    }

    #[test]
    fn international_text() {
        use super::parse;

        assert_eq!(
            "I&#x27;d bet it&#x27;s a &quot;test&quot;, yea.",
            parse("I'd bet it's a \"test\", yea.")
        );
        assert_eq!("私は猫<i>です</i>。", parse("私は猫[i]です[/i]。"));
        assert_eq!(
            "全世界無產階級和被壓迫的民族聯合起來！",
            parse("全世界無產階級和被壓迫的民族聯合起來！")
        );
        assert_eq!(
            "<b>СМЕРТЬ</b><br />ВСІМ, ХТО НА ПИРИШКОДІ<br />ДОБУТЬЯ ВІЛЬНОСТІ<br />ТРУДОВОМУ ЛЮДУ.",
            parse(
                "[b]СМЕРТЬ[/b]\r\nВСІМ, ХТО НА ПИРИШКОДІ\r\nДОБУТЬЯ ВІЛЬНОСТІ\r\nТРУДОВОМУ ЛЮДУ."
            )
        );
        assert_eq!("😂🔫", parse("😂🔫"));
    }

    #[test]
    fn invalid() {
        use super::parse;

        assert_eq!("[foo]Test[/foo]", parse("[foo]Test[/foo]"));
        assert_eq!("[foo]Test[/foo]", parse("[plain][foo]Test[/foo][/plain]"));
        assert_eq!("[foo]Test[/bar]", parse("[foo]Test[/bar]"));
        assert_eq!("[foo]Test", parse("[foo]Test"));
    }

    #[test]
    fn linebreaks() {
        use super::parse;

        assert_eq!("Foo<br />bar", parse("Foo\r\nbar"));
        assert_eq!("Foo<br />bar", parse("Foo\nbar"));
        assert_eq!("Foo<br />\rbar", parse("Foo\n\rbar"));
        assert_eq!("Foo<br />\rbar", parse("Foo\r\n\rbar"));

        assert_eq!("Foo<br /><br /><br />bar", parse("Foo\n\n\nbar"));
        assert_eq!(
            "<b>Foo<br /><br /><br />bar</b>",
            parse("[b]Foo\n\n\nbar[/b]")
        );
        assert_eq!("<b>Foo<br /><br /><br />bar</b>", parse("[b]Foo\n\n\nbar"));
    }

    #[test]
    fn linkify() {
        use super::parse;

        // Bare URLs auto-unfurl (new behavior)
        let bare_url = parse("Welcome, to https://zombo.com/");
        assert!(bare_url.contains("unfurl-container"));
        assert!(bare_url.contains("data-url=\"https://zombo.com/\""));

        // Explicit [url] tags render as plain links (no auto-unfurl)
        assert_eq!(
            "Welcome, to <a class=\"bbCode tagUrl\" rel=\"nofollow\" href=\"https://zombo.com/\">https://zombo.com/</a>!",
            parse("Welcome, to [url]https://zombo.com/[/url]!")
        );
        assert_eq!(
            "Welcome, to <b><a class=\"bbCode tagUrl\" rel=\"nofollow\" href=\"https://zombo.com/\">https://zombo.com/</a></b>!",
            parse("Welcome, to [b][url]https://zombo.com/[/url][/b]!")
        );

        // URL with display text - plain link
        assert_eq!(
            "Welcome, to <a class=\"bbCode tagUrl\" rel=\"nofollow\" href=\"https://zombo.com/\">Zombo.com</a>!",
            parse("Welcome, to [url=https://zombo.com/]Zombo.com[/url]!")
        );

        // URL with image inside - plain link
        assert_eq!(
            "<a class=\"bbCode tagUrl\" rel=\"nofollow\" href=\"https://zombo.com/\"><img src=\"https://zombo.com/images/zombocom.png\" /></a>",
            parse("[url=https://zombo.com/][img]https://zombo.com/images/zombocom.png[/img][/url]")
        );

        // [url unfurl] explicit unfurl
        let explicit_unfurl = parse("[url unfurl]https://zombo.com/[/url]");
        assert!(explicit_unfurl.contains("unfurl-container"));

        // [url nounfurl] disables unfurl
        let nounfurl = parse("[url nounfurl]https://zombo.com/[/url]");
        assert!(!nounfurl.contains("unfurl-container"));
        assert!(nounfurl.contains("<a class=\"bbCode tagUrl\""));

        // Empty/invalid URLs
        assert_eq!(
            "Welcome, to [url][/url]!",
            parse("Welcome, to [url][/url]!")
        );
        assert_eq!("Welcome, to [url]!", parse("Welcome, to [url]!"));
        assert_eq!("[url][/url]", parse("[url][/url]"));
        assert_eq!("[url]", parse("[url]"));
    }

    #[test]
    fn misc() {
        use super::parse;

        // This is a self-closing tag in HTML and I disagree that it should require a closing tag in BBCode.
        assert_eq!("<hr />", parse("[hr]"));
        //assert_eq!("<hr />", parse("[hr][/hr]"));
        assert_eq!("Foo<hr />Bar", parse("Foo[hr]Bar"));
        //assert_eq!("Foo<hr />Bar", parse("Foo[hr]Bar[/hr]"));
        //assert_eq!("Foo<hr />Bar", parse("Foo[hr][/hr]Bar"));
        assert_eq!("<b>Foo<hr />Bar</b>", parse("[b]Foo[hr]Bar"));
    }

    #[test]
    fn plain() {
        use super::parse;

        assert_eq!("[b]Test[/b]", parse("[plain][b]Test[/b][/plain]"));
        assert_eq!("[b]Test[/b]", parse("[plain][b]Test[/b]"));
        assert_eq!("[b]Foo[hr]bar[/b]", parse("[plain][b]Foo[hr]bar[/b]"));
    }

    #[test]
    fn pre() {
        use super::parse;

        assert_eq!("<pre><code>Test</code></pre>", parse("[code]Test[/code]"));
        assert_eq!(
            "<pre><code>Foo\r\nbar</code></pre>",
            parse("[code]Foo\r\nbar[/code]")
        );
        assert_eq!(
            "<pre><code>Foo\r\nbar&lt;/pre&gt;&lt;iframe&gt;</code></pre>",
            parse("[code]Foo\r\nbar</pre><iframe>[/code]")
        );
    }

    #[test]
    fn code_with_language() {
        use super::parse;

        // Valid languages with class
        assert_eq!(
            "<pre><code class=\"language-rust\">fn main() {}</code></pre>",
            parse("[code=rust]fn main() {}[/code]")
        );
        // Note: single quotes are escaped by sanitize()
        assert_eq!(
            "<pre><code class=\"language-javascript\">console.log(&#x27;hello&#x27;);</code></pre>",
            parse("[code=javascript]console.log('hello');[/code]")
        );
        assert_eq!(
            "<pre><code class=\"language-python\">print(&#x27;hello&#x27;)</code></pre>",
            parse("[code=python]print('hello')[/code]")
        );

        // Language aliases should be normalized
        assert_eq!(
            "<pre><code class=\"language-javascript\">let x = 1;</code></pre>",
            parse("[code=js]let x = 1;[/code]")
        );
        assert_eq!(
            "<pre><code class=\"language-python\">x = 1</code></pre>",
            parse("[code=py]x = 1[/code]")
        );
        assert_eq!(
            "<pre><code class=\"language-typescript\">const x: number = 1;</code></pre>",
            parse("[code=ts]const x: number = 1;[/code]")
        );
        assert_eq!(
            "<pre><code class=\"language-bash\">echo &#x27;hello&#x27;</code></pre>",
            parse("[code=sh]echo 'hello'[/code]")
        );

        // Invalid languages should fall back to no class
        assert_eq!(
            "<pre><code>some code</code></pre>",
            parse("[code=invalidlang]some code[/code]")
        );
        // XSS attempt in language should fall back to no class
        assert_eq!(
            "<pre><code>some code</code></pre>",
            parse("[code=malicious]some code[/code]")
        );

        // Case insensitive
        assert_eq!(
            "<pre><code class=\"language-rust\">code</code></pre>",
            parse("[code=RUST]code[/code]")
        );
        assert_eq!(
            "<pre><code class=\"language-javascript\">code</code></pre>",
            parse("[code=JavaScript]code[/code]")
        );

        // HTML content is still escaped
        assert_eq!(
            "<pre><code class=\"language-html\">&lt;div&gt;Hello&lt;/div&gt;</code></pre>",
            parse("[code=html]<div>Hello</div>[/code]")
        );
    }

    #[test]
    fn sanitize() {
        use super::parse;

        assert_eq!("&lt;b&gt;Test&lt;/b&gt;", parse("<b>Test</b>"));
        assert_eq!(
            "[xxx&lt;iframe&gt;]Test[/xxx&lt;iframe&gt;]",
            parse("[xxx<iframe>]Test[/xxx<iframe>]")
        );
        assert_eq!(
            "[url=javascript:alert(String.fromCharCode(88,83,83))]https://zombo.com[/url]",
            parse("[url=javascript:alert(String.fromCharCode(88,83,83))]https://zombo.com[/url]")
        )
    }

    #[test]
    fn lists() {
        use super::parse;

        // Unordered list
        assert_eq!(
            "<ul><li>Item 1</li><li>Item 2</li><li>Item 3</li></ul>",
            parse("[list][*]Item 1[*]Item 2[*]Item 3[/list]")
        );

        // Numbered list
        assert_eq!(
            "<ol type=\"1\"><li>First</li><li>Second</li><li>Third</li></ol>",
            parse("[list=1][*]First[*]Second[*]Third[/list]")
        );

        // Alphabetic list
        assert_eq!(
            "<ol type=\"a\"><li>Alpha</li><li>Beta</li><li>Gamma</li></ol>",
            parse("[list=a][*]Alpha[*]Beta[*]Gamma[/list]")
        );

        // Nested lists
        assert_eq!(
            "<ul><li>Item 1<ul><li>Subitem 1</li><li>Subitem 2</li></ul></li><li>Item 2</li></ul>",
            parse("[list][*]Item 1[list][*]Subitem 1[*]Subitem 2[/list][*]Item 2[/list]")
        );

        // Invalid list type
        assert_eq!(
            "[list=invalid][*]Item[/list]",
            parse("[list=invalid][*]Item[/list]")
        );

        // Lists with formatting
        assert_eq!(
            "<ul><li><b>Bold item</b></li><li><i>Italic item</i></li></ul>",
            parse("[list][*][b]Bold item[/b][*][i]Italic item[/i][/list]")
        );
    }

    #[test]
    fn spoilers() {
        use super::parse;

        // Basic spoiler with default title
        assert_eq!(
            "<details><summary>Spoiler</summary>Hidden content</details>",
            parse("[spoiler]Hidden content[/spoiler]")
        );

        // Spoiler with custom title
        assert_eq!(
            "<details><summary>Plot Twist</summary>The butler did it</details>",
            parse("[spoiler=Plot Twist]The butler did it[/spoiler]")
        );

        // Spoiler with formatting inside
        assert_eq!(
            "<details><summary>Spoiler</summary><b>Bold</b> and <i>italic</i> text</details>",
            parse("[spoiler][b]Bold[/b] and [i]italic[/i] text[/spoiler]")
        );

        // Nested spoilers
        assert_eq!(
            "<details><summary>Outer</summary>First level<details><summary>Inner</summary>Second level</details></details>",
            parse("[spoiler=Outer]First level[spoiler=Inner]Second level[/spoiler][/spoiler]")
        );

        // Empty spoiler
        assert_eq!(
            "<details><summary>Spoiler</summary></details>",
            parse("[spoiler][/spoiler]")
        );

        // HTML entity sanitization in title
        assert_eq!(
            "<details><summary>&quot;Quoted&quot; &amp; Safe</summary>Content</details>",
            parse("[spoiler=\"Quoted\" & Safe]Content[/spoiler]")
        );
    }

    #[test]
    fn alignment() {
        use super::parse;

        // Center alignment
        assert_eq!(
            "<div style=\"text-align: center;\">Centered text</div>",
            parse("[center]Centered text[/center]")
        );

        // Left alignment
        assert_eq!(
            "<div style=\"text-align: left;\">Left aligned text</div>",
            parse("[left]Left aligned text[/left]")
        );

        // Right alignment
        assert_eq!(
            "<div style=\"text-align: right;\">Right aligned text</div>",
            parse("[right]Right aligned text[/right]")
        );

        // Alignment with formatting inside
        assert_eq!(
            "<div style=\"text-align: center;\"><b>Bold</b> and <i>italic</i></div>",
            parse("[center][b]Bold[/b] and [i]italic[/i][/center]")
        );

        // Empty alignment
        assert_eq!(
            "<div style=\"text-align: center;\"></div>",
            parse("[center][/center]")
        );
    }

    #[test]
    fn quotes() {
        use super::parse;

        // Basic quote without attribution
        assert_eq!(
            "<blockquote class=\"bbCode tagQuote\"><div class=\"quoted\">This is a quote</div></blockquote>",
            parse("[quote]This is a quote[/quote]")
        );

        // Quote with attribution
        assert_eq!(
            "<blockquote class=\"bbCode tagQuote\" data-author=\"John Doe\"><div class=\"attribution\">John Doe said:</div><div class=\"quoted\">Hello world</div></blockquote>",
            parse("[quote=John Doe]Hello world[/quote]")
        );

        // Quote with formatting inside
        assert_eq!(
            "<blockquote class=\"bbCode tagQuote\"><div class=\"quoted\"><b>Bold</b> and <i>italic</i></div></blockquote>",
            parse("[quote][b]Bold[/b] and [i]italic[/i][/quote]")
        );

        // Nested quotes
        assert_eq!(
            "<blockquote class=\"bbCode tagQuote\" data-author=\"Alice\"><div class=\"attribution\">Alice said:</div><div class=\"quoted\">Outer<blockquote class=\"bbCode tagQuote\" data-author=\"Bob\"><div class=\"attribution\">Bob said:</div><div class=\"quoted\">Inner</div></blockquote></div></blockquote>",
            parse("[quote=Alice]Outer[quote=Bob]Inner[/quote][/quote]")
        );

        // Empty quote
        assert_eq!(
            "<blockquote class=\"bbCode tagQuote\"><div class=\"quoted\"></div></blockquote>",
            parse("[quote][/quote]")
        );

        // HTML sanitization in attribution
        assert_eq!(
            "<blockquote class=\"bbCode tagQuote\" data-author=\"&quot;User&quot; &amp; Co\"><div class=\"attribution\">&quot;User&quot; &amp; Co said:</div><div class=\"quoted\">Text</div></blockquote>",
            parse("[quote=\"User\" & Co]Text[/quote]")
        );
    }

    #[test]
    fn image_dimensions() {
        use super::parse;

        // Image with width and height
        assert_eq!(
            "<img src=\"https://example.com/image.jpg\" width=\"100\" height=\"150\" />",
            parse("[img=100x150]https://example.com/image.jpg[/img]")
        );

        // Image with just width (maintains aspect ratio)
        assert_eq!(
            "<img src=\"https://example.com/image.jpg\" width=\"200\" />",
            parse("[img=200]https://example.com/image.jpg[/img]")
        );

        // Image without dimensions (original behavior)
        assert_eq!(
            "<img src=\"https://example.com/image.jpg\" />",
            parse("[img]https://example.com/image.jpg[/img]")
        );

        // Invalid dimensions (too large)
        assert_eq!(
            "<img src=\"https://example.com/image.jpg\" />",
            parse("[img=5000x5000]https://example.com/image.jpg[/img]")
        );

        // Invalid dimensions (zero)
        assert_eq!(
            "<img src=\"https://example.com/image.jpg\" />",
            parse("[img=0x0]https://example.com/image.jpg[/img]")
        );

        // Invalid dimensions (malformed)
        assert_eq!(
            "<img src=\"https://example.com/image.jpg\" />",
            parse("[img=abcxdef]https://example.com/image.jpg[/img]")
        );

        // Maximum valid dimensions
        assert_eq!(
            "<img src=\"https://example.com/image.jpg\" width=\"2000\" height=\"2000\" />",
            parse("[img=2000x2000]https://example.com/image.jpg[/img]")
        );
    }

    #[test]
    fn mentions() {
        use super::parse;

        // Basic mention
        assert_eq!(
            "<a class=\"mention\" href=\"/members/@john\">@john</a>",
            parse("@john")
        );

        // Mention at start of text
        assert_eq!(
            "<a class=\"mention\" href=\"/members/@alice\">@alice</a> said hello",
            parse("@alice said hello")
        );

        // Mention after text
        assert_eq!(
            "Hello <a class=\"mention\" href=\"/members/@bob\">@bob</a>",
            parse("Hello @bob")
        );

        // Multiple mentions
        assert_eq!(
            "<a class=\"mention\" href=\"/members/@alice\">@alice</a> and <a class=\"mention\" href=\"/members/@bob\">@bob</a>",
            parse("@alice and @bob")
        );

        // Mention with underscore and hyphen
        assert_eq!(
            "<a class=\"mention\" href=\"/members/@user_name-123\">@user_name-123</a>",
            parse("@user_name-123")
        );

        // Mention with BBCode formatting
        assert_eq!(
            "<b><a class=\"mention\" href=\"/members/@john\">@john</a></b>",
            parse("[b]@john[/b]")
        );

        // Mention inside URL should NOT be linkified (already a link)
        // The URL tag creates an <a> so mentions inside shouldn't be processed
        let result = parse("[url=https://example.com]@notamentionhere[/url]");
        assert!(!result.contains("href=\"/members/@notamentionhere\""));

        // Mention inside code block should NOT be linkified
        assert_eq!(
            "<pre><code>@codemention</code></pre>",
            parse("[code]@codemention[/code]")
        );

        // Email-like text should NOT be a mention (has @ but preceded by text)
        assert_eq!("test@example.com", parse("test@example.com"));
    }

    #[test]
    fn media_embeds() {
        use super::parse;

        // YouTube embed with full URL
        let youtube_result = parse("[video]https://www.youtube.com/watch?v=dQw4w9WgXcQ[/video]");
        assert!(youtube_result.contains("youtube-nocookie.com/embed/dQw4w9WgXcQ"));
        assert!(youtube_result.contains("video-embed--youtube"));

        // YouTube short URL
        let youtube_short = parse("[video]https://youtu.be/dQw4w9WgXcQ[/video]");
        assert!(youtube_short.contains("youtube-nocookie.com/embed/dQw4w9WgXcQ"));

        // YouTube tag shorthand
        let youtube_tag = parse("[youtube]dQw4w9WgXcQ[/youtube]");
        assert!(youtube_tag.contains("youtube-nocookie.com/embed/dQw4w9WgXcQ"));

        // YouTube tag with full URL
        let youtube_tag_url = parse("[youtube]https://www.youtube.com/watch?v=abc123xyz[/youtube]");
        assert!(youtube_tag_url.contains("youtube-nocookie.com/embed/abc123xyz"));

        // Vimeo embed
        let vimeo_result = parse("[video]https://vimeo.com/123456789[/video]");
        assert!(vimeo_result.contains("player.vimeo.com/video/123456789"));
        assert!(vimeo_result.contains("video-embed--vimeo"));

        // Direct video file
        let video_file = parse("[video]https://example.com/video.mp4[/video]");
        assert!(video_file.contains("<video"));
        assert!(video_file.contains("video-embed--direct"));
        assert!(video_file.contains("example.com/video.mp4"));

        // Audio embed
        let audio_result = parse("[audio]https://example.com/audio.mp3[/audio]");
        assert!(audio_result.contains("<audio"));
        assert!(audio_result.contains("audio-embed"));
        assert!(audio_result.contains("example.com/audio.mp3"));

        // Media tag auto-detection - YouTube
        let media_yt = parse("[media]https://www.youtube.com/watch?v=test123[/media]");
        assert!(media_yt.contains("youtube-nocookie.com/embed/test123"));

        // Media tag auto-detection - direct video
        let media_video = parse("[media]https://example.com/clip.webm[/media]");
        assert!(media_video.contains("<video"));

        // Media tag auto-detection - audio
        let media_audio = parse("[media]https://example.com/song.mp3[/media]");
        assert!(media_audio.contains("<audio"));

        // Invalid URL should render as broken (text)
        let invalid = parse("[video]not-a-url[/video]");
        assert!(invalid.contains("[video]"));
        assert!(invalid.contains("[/video]"));

        // Invalid scheme should render as broken
        let invalid_scheme = parse("[video]ftp://example.com/video.mp4[/video]");
        assert!(invalid_scheme.contains("[video]"));
    }

    #[test]
    fn tables() {
        use super::parse;

        // Basic table structure
        let basic_table = parse("[table][tr][td]Cell 1[/td][td]Cell 2[/td][/tr][/table]");
        assert!(basic_table.contains("<table class=\"bbcode-table\">"));
        assert!(basic_table.contains("<tr>"));
        assert!(basic_table.contains("<td>Cell 1</td>"));
        assert!(basic_table.contains("<td>Cell 2</td>"));
        assert!(basic_table.contains("</tr>"));
        assert!(basic_table.contains("</table>"));

        // Table with header cells
        let table_headers = parse("[table][tr][th]Header 1[/th][th]Header 2[/th][/tr][tr][td]Data 1[/td][td]Data 2[/td][/tr][/table]");
        assert!(table_headers.contains("<th>Header 1</th>"));
        assert!(table_headers.contains("<th>Header 2</th>"));
        assert!(table_headers.contains("<td>Data 1</td>"));
        assert!(table_headers.contains("<td>Data 2</td>"));

        // Multi-row table
        let multi_row = parse("[table][tr][td]R1C1[/td][/tr][tr][td]R2C1[/td][/tr][/table]");
        assert!(multi_row.contains("<td>R1C1</td>"));
        assert!(multi_row.contains("<td>R2C1</td>"));

        // Table with formatting inside cells
        let formatted_table = parse("[table][tr][td][b]Bold text[/b][/td][/tr][/table]");
        assert!(formatted_table.contains("<td><b>Bold text</b></td>"));

        // Invalid: [tr] outside of [table] should be broken
        let invalid_tr = parse("[tr][td]No table[/td][/tr]");
        assert!(invalid_tr.contains("[tr]"));

        // Invalid: [td] outside of [tr] should be broken
        let invalid_td = parse("[table][td]No row[/td][/table]");
        assert!(invalid_td.contains("[td]"));

        // Invalid: [th] outside of [tr] should be broken
        let invalid_th = parse("[th]No row[/th]");
        assert!(invalid_th.contains("[th]"));

        // Auto-close previous cell when opening new cell
        let auto_close = parse("[table][tr][td]Cell 1[td]Cell 2[/td][/tr][/table]");
        assert!(auto_close.contains("<td>Cell 1</td>"));
        assert!(auto_close.contains("<td>Cell 2</td>"));
    }
}
