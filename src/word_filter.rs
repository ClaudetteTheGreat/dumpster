//! Word filter system for content moderation
//!
//! This module provides functionality to filter user-generated content based on
//! administrator-defined patterns. Supports three actions:
//!
//! - **Replace**: Substitute matched text with a replacement (word exchange)
//! - **Block**: Reject the content entirely
//! - **Flag**: Allow content but mark it for moderator review

use crate::orm::word_filters::{self, FilterAction};
use once_cell::sync::OnceCell;
use regex::Regex;
use sea_orm::{entity::*, query::*, DatabaseConnection};
use std::sync::RwLock;

/// Result of filtering content
#[derive(Debug, Clone)]
pub struct FilterResult {
    /// The (possibly modified) content after filtering
    pub content: String,
    /// Whether the content was blocked
    pub blocked: bool,
    /// Whether the content was flagged for review
    pub flagged: bool,
    /// Patterns that matched (for logging/reporting)
    pub matched_patterns: Vec<String>,
    /// Block reason if content was blocked
    pub block_reason: Option<String>,
}

impl FilterResult {
    /// Create a result for content that passed filtering unchanged
    pub fn passed(content: String) -> Self {
        Self {
            content,
            blocked: false,
            flagged: false,
            matched_patterns: Vec::new(),
            block_reason: None,
        }
    }
}

/// Compiled word filter for efficient matching
#[derive(Debug)]
#[allow(dead_code)]
struct CompiledFilter {
    id: i32,
    pattern: String,
    replacement: Option<String>,
    action: FilterAction,
    regex: Option<Regex>,
    is_case_sensitive: bool,
    is_whole_word: bool,
}

impl CompiledFilter {
    fn from_model(model: &word_filters::Model) -> Option<Self> {
        let regex = if model.is_regex {
            // Compile regex pattern
            let pattern = if model.is_case_sensitive {
                model.pattern.clone()
            } else {
                format!("(?i){}", model.pattern)
            };
            match Regex::new(&pattern) {
                Ok(r) => Some(r),
                Err(e) => {
                    log::error!("Failed to compile word filter regex {}: {}", model.id, e);
                    return None;
                }
            }
        } else {
            None
        };

        Some(Self {
            id: model.id,
            pattern: model.pattern.clone(),
            replacement: model.replacement.clone(),
            action: model.action.clone(),
            regex,
            is_case_sensitive: model.is_case_sensitive,
            is_whole_word: model.is_whole_word,
        })
    }

    /// Check if this filter matches the given content and return match positions
    fn find_matches(&self, content: &str) -> Vec<(usize, usize)> {
        let mut matches = Vec::new();

        if let Some(ref regex) = self.regex {
            // Regex matching
            for m in regex.find_iter(content) {
                if self.is_whole_word {
                    // Check word boundaries manually for regex
                    let start = m.start();
                    let end = m.end();
                    if is_word_boundary(content, start, end) {
                        matches.push((start, end));
                    }
                } else {
                    matches.push((m.start(), m.end()));
                }
            }
        } else {
            // Plain text matching
            let search_content = if self.is_case_sensitive {
                content.to_string()
            } else {
                content.to_lowercase()
            };
            let search_pattern = if self.is_case_sensitive {
                self.pattern.clone()
            } else {
                self.pattern.to_lowercase()
            };

            let mut start = 0;
            while let Some(pos) = search_content[start..].find(&search_pattern) {
                let match_start = start + pos;
                let match_end = match_start + self.pattern.len();

                if self.is_whole_word {
                    if is_word_boundary(content, match_start, match_end) {
                        matches.push((match_start, match_end));
                    }
                } else {
                    matches.push((match_start, match_end));
                }

                start = match_end;
            }
        }

        matches
    }
}

/// Check if positions represent word boundaries
fn is_word_boundary(content: &str, start: usize, end: usize) -> bool {
    let chars: Vec<char> = content.chars().collect();
    let byte_to_char: Vec<usize> = content
        .char_indices()
        .map(|(i, _)| i)
        .chain(std::iter::once(content.len()))
        .collect();

    // Find character indices
    let start_char = byte_to_char.iter().position(|&b| b == start);
    let end_char = byte_to_char.iter().position(|&b| b == end);

    let (start_char, end_char) = match (start_char, end_char) {
        (Some(s), Some(e)) => (s, e),
        _ => return false,
    };

    // Check start boundary
    let start_ok = start_char == 0 || !chars[start_char - 1].is_alphanumeric();

    // Check end boundary
    let end_ok = end_char >= chars.len() || !chars[end_char].is_alphanumeric();

    start_ok && end_ok
}

/// Global filter cache
static FILTER_CACHE: OnceCell<RwLock<Vec<CompiledFilter>>> = OnceCell::new();

/// Initialize the filter cache from the database
pub async fn init_filters(db: &DatabaseConnection) -> Result<(), sea_orm::DbErr> {
    let filters = word_filters::Entity::find()
        .filter(word_filters::Column::IsEnabled.eq(true))
        .all(db)
        .await?;

    let compiled: Vec<CompiledFilter> = filters
        .iter()
        .filter_map(CompiledFilter::from_model)
        .collect();

    log::info!("Loaded {} word filters", compiled.len());

    let cache = FILTER_CACHE.get_or_init(|| RwLock::new(Vec::new()));
    let mut cache_write = cache.write().unwrap();
    *cache_write = compiled;

    Ok(())
}

/// Reload filters from database (call after adding/editing filters)
pub async fn reload_filters(db: &DatabaseConnection) -> Result<(), sea_orm::DbErr> {
    init_filters(db).await
}

/// Apply word filters to content
///
/// Returns a FilterResult containing the (possibly modified) content and
/// information about any matches.
pub fn apply_filters(content: &str) -> FilterResult {
    let cache = match FILTER_CACHE.get() {
        Some(c) => c,
        None => return FilterResult::passed(content.to_string()),
    };

    let filters = match cache.read() {
        Ok(f) => f,
        Err(_) => return FilterResult::passed(content.to_string()),
    };

    if filters.is_empty() {
        return FilterResult::passed(content.to_string());
    }

    let mut result_content = content.to_string();
    let mut blocked = false;
    let mut flagged = false;
    let mut matched_patterns = Vec::new();
    let mut block_reason = None;

    // Process filters by action priority: block first, then flag, then replace
    // This ensures blocking takes precedence

    // First pass: check for blocks
    for filter in filters.iter() {
        if filter.action == FilterAction::Block {
            let matches = filter.find_matches(&result_content);
            if !matches.is_empty() {
                blocked = true;
                matched_patterns.push(filter.pattern.clone());
                block_reason = Some(format!("Content contains blocked word: {}", filter.pattern));
                break; // One block is enough
            }
        }
    }

    // If blocked, return early
    if blocked {
        return FilterResult {
            content: result_content,
            blocked,
            flagged,
            matched_patterns,
            block_reason,
        };
    }

    // Second pass: check for flags
    for filter in filters.iter() {
        if filter.action == FilterAction::Flag {
            let matches = filter.find_matches(&result_content);
            if !matches.is_empty() {
                flagged = true;
                matched_patterns.push(filter.pattern.clone());
            }
        }
    }

    // Third pass: apply replacements
    for filter in filters.iter() {
        if filter.action == FilterAction::Replace {
            if let Some(ref replacement) = filter.replacement {
                let matches = filter.find_matches(&result_content);
                if !matches.is_empty() {
                    matched_patterns.push(filter.pattern.clone());

                    // Apply replacements in reverse order to preserve positions
                    let mut sorted_matches = matches.clone();
                    sorted_matches.sort_by(|a, b| b.0.cmp(&a.0));

                    for (start, end) in sorted_matches {
                        // Preserve case if replacement is provided
                        let original = &result_content[start..end];
                        let new_replacement = match_case(original, replacement);
                        result_content.replace_range(start..end, &new_replacement);
                    }
                }
            }
        }
    }

    FilterResult {
        content: result_content,
        blocked,
        flagged,
        matched_patterns,
        block_reason,
    }
}

/// Match the case pattern of the original text in the replacement
fn match_case(original: &str, replacement: &str) -> String {
    if original
        .chars()
        .all(|c| c.is_uppercase() || !c.is_alphabetic())
    {
        // All uppercase
        replacement.to_uppercase()
    } else if original
        .chars()
        .next()
        .map(|c| c.is_uppercase())
        .unwrap_or(false)
    {
        // Title case (first letter uppercase)
        let mut chars = replacement.chars();
        match chars.next() {
            None => String::new(),
            Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        }
    } else {
        // Lowercase or mixed - use replacement as-is
        replacement.to_string()
    }
}

/// Check if content would be blocked by filters (without applying replacements)
pub fn would_block(content: &str) -> Option<String> {
    let cache = FILTER_CACHE.get()?;

    let filters = match cache.read() {
        Ok(f) => f,
        Err(_) => return None,
    };

    for filter in filters.iter() {
        if filter.action == FilterAction::Block {
            let matches = filter.find_matches(content);
            if !matches.is_empty() {
                return Some(format!("Content contains blocked word: {}", filter.pattern));
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_match_case_uppercase() {
        assert_eq!(match_case("HELLO", "world"), "WORLD");
    }

    #[test]
    fn test_match_case_titlecase() {
        assert_eq!(match_case("Hello", "world"), "World");
    }

    #[test]
    fn test_match_case_lowercase() {
        assert_eq!(match_case("hello", "World"), "World");
    }

    #[test]
    fn test_word_boundary_start() {
        assert!(is_word_boundary("hello world", 0, 5));
    }

    #[test]
    fn test_word_boundary_middle() {
        assert!(is_word_boundary("hello world", 6, 11));
    }

    #[test]
    fn test_word_boundary_not_word() {
        assert!(!is_word_boundary("helloworld", 0, 5));
    }
}
