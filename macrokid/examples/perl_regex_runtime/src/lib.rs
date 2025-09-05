// Runtime support types for Perl-like regex DSL

pub use regex::Regex;
use regex::Captures;

pub struct PerlRegexMatch {
    pub matched: bool,
    pub full_text: String,
    captures: Option<String>, // We'll store capture text since Captures has lifetime issues
}

impl PerlRegexMatch {
    pub fn new_match(text: &str, caps: Captures) -> Self {
        Self {
            matched: true,
            full_text: text.to_string(),
            captures: caps.get(0).map(|m| m.as_str().to_string()),
        }
    }

    pub fn no_match(text: &str) -> Self {
        Self {
            matched: false,
            full_text: text.to_string(),
            captures: None,
        }
    }

    /// Get the full match (Perl's $&)
    pub fn full_match(&self) -> Option<&str> {
        self.captures.as_deref()
    }
}

pub struct PerlRegexSubst {
    pub result: String,
    pub count: usize,  // Number of substitutions made
}

impl std::fmt::Display for PerlRegexSubst {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.result)
    }
}
