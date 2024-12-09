use ignore::gitignore::{Gitignore, GitignoreBuilder};
use std::path::Path;

pub struct IgnoreFilesHelper {
    gitignore: Option<Gitignore>,
    custom_ignore: Option<Gitignore>,
}

impl IgnoreFilesHelper {
    pub fn new() -> Self {
        let gitignore = {
            let mut builder = GitignoreBuilder::new(".");
            match builder.add(".gitignore") {
                None => builder.build().ok(),
                Some(_) => None,
            }
        };

        let custom_ignore = {
            let mut builder = GitignoreBuilder::new(".");
            match builder.add("to_ignore") {
                None => builder.build().ok(),
                Some(_) => None,
            }
        };

        Self {
            gitignore,
            custom_ignore,
        }
    }

    pub fn is_ignored(&self, path: &Path) -> bool {
        let is_dir = path.is_dir();

        // Check custom ignore first
        if let Some(ci) = &self.custom_ignore {
            if ci.matched(path, is_dir).is_ignore() {
                return true;
            }
        }

        // Then check gitignore if needed
        if let Some(gi) = &self.gitignore {
            if gi.matched(path, is_dir).is_ignore() {
                return true;
            }
        }

        false
    }
}