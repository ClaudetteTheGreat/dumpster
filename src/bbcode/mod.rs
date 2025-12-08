extern crate linkify;

mod constructor;
mod element;
mod parser;
mod smilie;
mod tag;
mod token;
mod tokenize;

pub use constructor::Constructor;
pub use element::{Element, ElementDisplay};
pub use parser::Parser;
pub use smilie::Smilies;
pub use tag::Tag;
pub use token::Token;
pub use tokenize::tokenize;

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
    constructor.build(ast)
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

        assert_eq!(
            "Welcome, to <a class=\"bbCode tagUrl\" ref=\"nofollow\" href=\"https://zombo.com/\">https://zombo.com/</a>",
            parse("Welcome, to https://zombo.com/")
        );
        assert_eq!(
            "Welcome, to <a class=\"bbCode tagUrl\" ref=\"nofollow\" href=\"https://zombo.com/\">https://zombo.com/</a>!",
            parse("Welcome, to [url]https://zombo.com/[/url]!")
        );
        assert_eq!(
            "Welcome, to <b><a class=\"bbCode tagUrl\" ref=\"nofollow\" href=\"https://zombo.com/\">https://zombo.com/</a></b>!",
            parse("Welcome, to [b][url]https://zombo.com/[/url][/b]!")
        );
        assert_eq!(
            "Welcome, to <a class=\"bbCode tagUrl\" ref=\"nofollow\" href=\"https://zombo.com/\">Zombo.com</a>!",
            parse("Welcome, to [url=https://zombo.com/]Zombo.com[/url]!")
        );
        assert_eq!(
            "<a class=\"bbCode tagUrl\" ref=\"nofollow\" href=\"https://zombo.com/\"><img src=\"https://zombo.com/images/zombocom.png\" /></a>",
            parse("[url=https://zombo.com/][img]https://zombo.com/images/zombocom.png[/img][/url]")
        );
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

        assert_eq!("<pre>Test</pre>", parse("[code]Test[/code]"));
        assert_eq!("<pre>Foo\r\nbar</pre>", parse("[code]Foo\r\nbar[/code]"));
        assert_eq!("<pre>Foo\r\nbar&lt;/pre&gt;&lt;iframe&gt;</pre>", parse("[code]Foo\r\nbar</pre><iframe>[/code]"));
    }

    #[test]
    fn sanitize() {
        use super::parse;

        assert_eq!("&lt;b&gt;Test&lt;/b&gt;", parse("<b>Test</b>"));
        assert_eq!("[xxx&lt;iframe&gt;]Test[/xxx&lt;iframe&gt;]", parse("[xxx<iframe>]Test[/xxx<iframe>]"));
        assert_eq!("[url=javascript:alert(String.fromCharCode(88,83,83))]https://zombo.com[/url]", parse("[url=javascript:alert(String.fromCharCode(88,83,83))]https://zombo.com[/url]"))
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
}
