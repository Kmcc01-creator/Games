// Runtime support types for Perl-like regex DSL

pub use regex::Regex;
use regex::Captures;

pub struct PerlRegexMatch {
    pub matched: bool,
    pub full_text: String,
    captures: Option<String>, // Full match (group 0) when not using /g; stored to avoid lifetime issues
    all: Vec<String>,         // When using global flag, collect all full matches (group 0)
}

impl PerlRegexMatch {
    pub fn new_match(text: &str, caps: Captures) -> Self {
        Self {
            matched: true,
            full_text: text.to_string(),
            captures: caps.get(0).map(|m| m.as_str().to_string()),
            all: caps.get(0).map(|m| vec![m.as_str().to_string()]).unwrap_or_default(),
        }
    }

    pub fn no_match(text: &str) -> Self {
        Self {
            matched: false,
            full_text: text.to_string(),
            captures: None,
            all: Vec::new(),
        }
    }

    /// Get the full match (Perl's $&)
    pub fn full_match(&self) -> Option<&str> {
        self.captures.as_deref()
    }

    /// All full matches (group 0) when using the /g flag; empty otherwise.
    pub fn all_matches(&self) -> &[String] {
        &self.all
    }

    /// Build from a list of full matches (for the /g flag).
    pub fn from_all(text: &str, all: Vec<String>) -> Self {
        let captures = all.get(0).cloned();
        Self { matched: !all.is_empty(), full_text: text.to_string(), captures, all }
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
