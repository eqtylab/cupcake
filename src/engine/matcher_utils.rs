/// Utilities for matcher pattern detection and evaluation
use regex::Regex;

/// Determine if a matcher string should be treated as a regex pattern.
///
/// Per the tactical advisory: If a string contains metacharacters like
/// `|`, `(`, `[`, `*`, `+`, `?`, `^`, `$`, then treat it as a regex.
/// Otherwise, it is a literal string for exact matching.
///
/// This conservative approach prevents users from accidentally writing
/// broad matchers when they intend exact matches.
pub fn is_regex(matcher: &str) -> bool {
    // Check for common regex metacharacters
    matcher.contains('|')
        || matcher.contains('(')
        || matcher.contains('[')
        || matcher.contains('*')
        || matcher.contains('+')
        || matcher.contains('?')
        || matcher.contains('^')
        || matcher.contains('$')
        || matcher.contains('\\')
        || matcher.contains('{')
        || matcher.contains('.')
}

/// Evaluate if a matcher matches a query string.
///
/// Rules:
/// - If matcher is "*" or empty string, it matches everything
/// - If matcher contains regex metacharacters, compile and match as regex
/// - Otherwise, perform exact string match
pub fn evaluate_matcher(matcher: &str, query: &str) -> Result<bool, crate::CupcakeError> {
    // Wildcard and empty string match everything
    if matcher == "*" || matcher.is_empty() {
        return Ok(true);
    }

    // Check if this should be treated as regex
    if is_regex(matcher) {
        // Compile and match as regex
        let matcher_regex = Regex::new(matcher).map_err(|e| {
            crate::CupcakeError::Config(format!("Invalid matcher regex '{matcher}': {e}"))
        })?;
        Ok(matcher_regex.is_match(query))
    } else {
        // Exact string match
        Ok(matcher == query)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_regex_detection() {
        // Should be treated as regex
        assert!(is_regex("Bash|Edit"));
        assert!(is_regex("(Bash|Edit)"));
        assert!(is_regex("[BE]ash"));
        assert!(is_regex("Bash*"));
        assert!(is_regex("Bash+"));
        assert!(is_regex("Bash?"));
        assert!(is_regex("^Bash"));
        assert!(is_regex("Bash$"));
        assert!(is_regex("Ba.h"));
        assert!(is_regex("Bash\\s"));
        assert!(is_regex("Bash{1,3}"));

        // Should NOT be treated as regex (exact match)
        assert!(!is_regex("Bash"));
        assert!(!is_regex("Edit"));
        assert!(!is_regex("BashScript"));
        assert!(!is_regex("Read-File"));
        assert!(!is_regex("tool_name_123"));
    }

    #[test]
    fn test_evaluate_matcher_wildcard() {
        assert!(evaluate_matcher("*", "Bash").unwrap());
        assert!(evaluate_matcher("*", "Edit").unwrap());
        assert!(evaluate_matcher("*", "anything").unwrap());

        assert!(evaluate_matcher("", "Bash").unwrap());
        assert!(evaluate_matcher("", "Edit").unwrap());
        assert!(evaluate_matcher("", "anything").unwrap());
    }

    #[test]
    fn test_evaluate_matcher_exact() {
        // Exact matches
        assert!(evaluate_matcher("Bash", "Bash").unwrap());
        assert!(evaluate_matcher("Edit", "Edit").unwrap());

        // Non-matches
        assert!(!evaluate_matcher("Bash", "BashScript").unwrap());
        assert!(!evaluate_matcher("Bash", "bash").unwrap()); // Case sensitive
        assert!(!evaluate_matcher("Edit", "EditFile").unwrap());
    }

    #[test]
    fn test_evaluate_matcher_regex() {
        // Regex patterns
        assert!(evaluate_matcher("Bash|Edit", "Bash").unwrap());
        assert!(evaluate_matcher("Bash|Edit", "Edit").unwrap());
        assert!(!evaluate_matcher("Bash|Edit", "Read").unwrap());

        assert!(evaluate_matcher("Ba.h", "Bash").unwrap());
        assert!(evaluate_matcher("Ba.h", "Bath").unwrap());
        assert!(!evaluate_matcher("Ba.h", "Batch").unwrap());

        assert!(evaluate_matcher("^Bash", "Bash").unwrap());
        assert!(evaluate_matcher("^Bash", "BashScript").unwrap());
        assert!(!evaluate_matcher("^Bash", "MyBash").unwrap());
    }

    #[test]
    fn test_evaluate_matcher_invalid_regex() {
        // Invalid regex should return error
        let result = evaluate_matcher("[unclosed", "test");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Invalid matcher regex"));
    }
}
