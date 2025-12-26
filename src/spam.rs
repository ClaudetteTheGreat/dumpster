//! Spam detection module for content analysis
//!
//! Provides heuristic-based spam detection for posts and other user content.
//! Uses a scoring system where higher scores indicate more likely spam.

use once_cell::sync::Lazy;
use regex::Regex;

/// URL pattern for detecting links in content
static URL_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"https?://[^\s\])<>]+").expect("Invalid URL regex"));

/// Common spam phrases to check for
const SPAM_PHRASES: &[&str] = &[
    "click here",
    "buy now",
    "limited time",
    "make money",
    "work from home",
    "earn money",
    "free money",
    "act now",
    "order now",
    "special offer",
    "you've been selected",
    "congratulations you won",
    "claim your prize",
    "crypto investment",
    "bitcoin opportunity",
];

/// Result of spam analysis
#[derive(Debug, Clone)]
pub struct SpamAnalysis {
    /// Spam score from 0.0 (not spam) to 1.0+ (definitely spam)
    pub score: f32,
    /// Whether the content is considered spam (score >= threshold)
    pub is_spam: bool,
    /// Reasons contributing to the spam score
    pub reasons: Vec<String>,
}

impl SpamAnalysis {
    /// Default spam threshold
    const THRESHOLD: f32 = 0.7;

    /// Create a new spam analysis result
    fn new(score: f32, reasons: Vec<String>) -> Self {
        Self {
            score,
            is_spam: score >= Self::THRESHOLD,
            reasons,
        }
    }
}

/// Analyze content for spam indicators
///
/// # Arguments
/// * `content` - The text content to analyze
/// * `user_post_count` - Number of posts the user has made (0 for new users)
///
/// # Returns
/// SpamAnalysis with score, is_spam flag, and reasons
pub fn analyze_content(content: &str, user_post_count: i32) -> SpamAnalysis {
    let mut score = 0.0f32;
    let mut reasons = Vec::new();

    // Skip very short content
    if content.len() < 5 {
        return SpamAnalysis::new(0.0, reasons);
    }

    let content_lower = content.to_lowercase();

    // Check for excessive URLs
    let url_count = URL_REGEX.find_iter(content).count();
    if url_count > 5 {
        score += 0.4;
        reasons.push(format!("Excessive URLs: {}", url_count));
    } else if url_count > 3 {
        score += 0.2;
        reasons.push(format!("Multiple URLs: {}", url_count));
    }

    // New user posting links is suspicious
    if user_post_count == 0 && url_count > 0 {
        score += 0.3;
        reasons.push("First post contains URLs".to_string());
    }

    // Check for repeated characters (e.g., "aaaaaaa")
    if has_repeated_characters(content, 5) {
        score += 0.15;
        reasons.push("Repeated characters detected".to_string());
    }

    // Check for ALL CAPS (only for longer content)
    if content.len() > 30 {
        let alpha_chars: Vec<char> = content.chars().filter(|c| c.is_alphabetic()).collect();
        if !alpha_chars.is_empty() {
            let caps_ratio =
                alpha_chars.iter().filter(|c| c.is_uppercase()).count() as f32
                    / alpha_chars.len() as f32;

            if caps_ratio > 0.8 {
                score += 0.25;
                reasons.push("Excessive capitalization".to_string());
            } else if caps_ratio > 0.6 {
                score += 0.1;
                reasons.push("High capitalization".to_string());
            }
        }
    }

    // Check for common spam phrases
    for phrase in SPAM_PHRASES {
        if content_lower.contains(phrase) {
            score += 0.3;
            reasons.push(format!("Contains spam phrase: '{}'", phrase));
            break; // Only count once
        }
    }

    // Very short post with URL is suspicious
    if content.len() < 50 && url_count > 0 {
        score += 0.2;
        reasons.push("Short post with URL".to_string());
    }

    // Check for excessive punctuation
    let punct_count = content.chars().filter(|c| *c == '!' || *c == '?').count();
    if punct_count > 10 {
        score += 0.15;
        reasons.push(format!("Excessive punctuation: {}", punct_count));
    }

    // Check for emoji spam (multiple emoji in short content)
    let emoji_count = content
        .chars()
        .filter(|c| {
            let code = *c as u32;
            // Common emoji ranges
            (0x1F600..=0x1F64F).contains(&code)  // Emoticons
                || (0x1F300..=0x1F5FF).contains(&code) // Misc Symbols
                || (0x1F680..=0x1F6FF).contains(&code) // Transport
                || (0x2600..=0x26FF).contains(&code)   // Misc symbols
        })
        .count();

    if emoji_count > 10 && content.len() < 200 {
        score += 0.15;
        reasons.push(format!("Excessive emoji: {}", emoji_count));
    }

    SpamAnalysis::new(score, reasons)
}

/// Quick check if content might be spam (for pre-filtering)
///
/// This is a faster check that only looks at the most obvious indicators.
pub fn quick_spam_check(content: &str) -> bool {
    // Very obvious spam indicators
    let url_count = URL_REGEX.find_iter(content).count();
    if url_count > 10 {
        return true;
    }

    let content_lower = content.to_lowercase();
    for phrase in &SPAM_PHRASES[..5] {
        // Only check top phrases
        if content_lower.contains(phrase) && url_count > 0 {
            return true;
        }
    }

    false
}

/// Check if content has repeated characters (e.g., "aaaaaaa")
fn has_repeated_characters(content: &str, threshold: usize) -> bool {
    let mut prev_char = '\0';
    let mut count = 1;

    for c in content.chars() {
        if c == prev_char && c.is_alphabetic() {
            count += 1;
            if count >= threshold {
                return true;
            }
        } else {
            count = 1;
            prev_char = c;
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clean_content() {
        let result = analyze_content("This is a normal post about a topic.", 10);
        assert!(!result.is_spam);
        assert!(result.score < 0.3);
    }

    #[test]
    fn test_url_spam() {
        let content = "Check out http://spam1.com http://spam2.com http://spam3.com http://spam4.com http://spam5.com http://spam6.com";
        let result = analyze_content(content, 10);
        assert!(result.score >= 0.4);
        assert!(result.reasons.iter().any(|r| r.contains("URL")));
    }

    #[test]
    fn test_new_user_with_url() {
        let result = analyze_content("Check this out: http://example.com", 0);
        assert!(result.score >= 0.3);
        assert!(result.reasons.iter().any(|r| r.contains("First post")));
    }

    #[test]
    fn test_all_caps() {
        let result = analyze_content("THIS IS ALL CAPS AND IT'S VERY ANNOYING TO READ", 10);
        assert!(result.score >= 0.1);
        assert!(result
            .reasons
            .iter()
            .any(|r| r.contains("capitalization")));
    }

    #[test]
    fn test_spam_phrase() {
        let result = analyze_content("Click here to make money fast!", 10);
        assert!(result.score >= 0.3);
        assert!(result.reasons.iter().any(|r| r.contains("spam phrase")));
    }

    #[test]
    fn test_repeated_chars() {
        let result = analyze_content("Wooooooow this is amaziiiiing", 10);
        assert!(result.score >= 0.1);
        assert!(result.reasons.iter().any(|r| r.contains("Repeated")));
    }

    #[test]
    fn test_combined_spam() {
        // Content with multiple spam indicators: new user, multiple URLs, spam phrase, all caps
        let content = "CLICK HERE NOW!!! BUY NOW!!! http://spam1.com http://spam2.com http://spam3.com http://spam4.com";
        let result = analyze_content(content, 0);
        assert!(result.is_spam, "Score was {:.2}, reasons: {:?}", result.score, result.reasons);
        assert!(result.score >= 0.7);
    }

    #[test]
    fn test_quick_check_clean() {
        assert!(!quick_spam_check("Normal post content"));
    }

    #[test]
    fn test_quick_check_spam() {
        let spam = "Click here http://spam.com to win!";
        assert!(quick_spam_check(spam));
    }
}
