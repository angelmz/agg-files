use regex::Regex;

pub struct PatternMatcher;

impl PatternMatcher {
    pub fn new() -> Self {
        Self
    }

    pub fn glob_to_regex(&self, pattern: &str) -> Regex {
        let regex_str = pattern
            .replace(".", "\\.")
            .replace("*", ".*")
            .replace("{", "(")
            .replace("}", ")")
            .replace(",", "|")
            .replace(" ", "");  // Remove spaces
        
        Regex::new(&format!(".*{}$", regex_str)).unwrap()
    }
}
