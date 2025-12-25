use super::Element;
use std::cell::RefMut;

/// Whitelist of supported languages for syntax highlighting
/// Using highlight.js language identifiers
const SUPPORTED_LANGUAGES: &[&str] = &[
    // Common web languages
    "html",
    "css",
    "javascript",
    "js",
    "typescript",
    "ts",
    "json",
    "xml",
    // Backend languages
    "rust",
    "python",
    "py",
    "ruby",
    "rb",
    "php",
    "go",
    "java",
    "kotlin",
    "scala",
    "c",
    "cpp",
    "csharp",
    "cs",
    "swift",
    "objectivec",
    // Shell and config
    "bash",
    "shell",
    "sh",
    "zsh",
    "powershell",
    "ps1",
    "yaml",
    "yml",
    "toml",
    "ini",
    "dockerfile",
    // Database
    "sql",
    "mysql",
    "postgresql",
    "pgsql",
    // Markup and data
    "markdown",
    "md",
    "latex",
    "tex",
    // Other
    "lua",
    "perl",
    "r",
    "haskell",
    "elixir",
    "erlang",
    "clojure",
    "lisp",
    "scheme",
    "ocaml",
    "fsharp",
    "asm",
    "nasm",
    "wasm",
    "makefile",
    "cmake",
    "nginx",
    "apache",
    "diff",
    "patch",
    "plaintext",
    "text",
    "plain",
];

/// Normalize language aliases to canonical names
fn normalize_language(lang: &str) -> String {
    match lang.to_lowercase().as_str() {
        "js" => "javascript".to_string(),
        "ts" => "typescript".to_string(),
        "py" => "python".to_string(),
        "rb" => "ruby".to_string(),
        "cs" => "csharp".to_string(),
        "sh" => "bash".to_string(),
        "yml" => "yaml".to_string(),
        "md" => "markdown".to_string(),
        "ps1" => "powershell".to_string(),
        "pgsql" => "postgresql".to_string(),
        "tex" => "latex".to_string(),
        "text" | "plain" => "plaintext".to_string(),
        _ => lang.to_lowercase(),
    }
}

/// Check if a language is supported
fn is_valid_language(lang: &str) -> bool {
    let normalized = lang.to_lowercase();
    SUPPORTED_LANGUAGES.iter().any(|&l| l == normalized)
}

impl super::Tag {
    /// Opens a code block with optional language for syntax highlighting
    /// [code] or [code=language]
    pub fn open_code_tag(el: RefMut<Element>) -> String {
        if let Some(arg) = el.get_argument() {
            let lang = arg.strip_prefix('=').unwrap_or(arg).trim();

            if !lang.is_empty() && is_valid_language(lang) {
                let normalized = normalize_language(lang);
                return format!("<pre><code class=\"language-{}\">", normalized);
            }
        }

        // No language specified or invalid language
        String::from("<pre><code>")
    }

    /// Closes a code block
    pub fn close_code_tag() -> String {
        String::from("</code></pre>")
    }
}
