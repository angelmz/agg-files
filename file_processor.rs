use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use crate::cli::CliArgs;
use crate::gitignore_helper::GitignoreHelper;
use crate::pattern_matcher::PatternMatcher;

pub struct FileProcessor {
    args: CliArgs,
    gitignore: Option<ignore::gitignore::Gitignore>,
    pattern_matcher: PatternMatcher,
    working_dir: PathBuf,
}

impl FileProcessor {
    pub fn new(args: CliArgs, working_dir: PathBuf) -> Self {
        let gitignore = if !args.ignore_gitignore {
            GitignoreHelper::build()
        } else {
            None
        };

        Self {
            args,
            gitignore,
            pattern_matcher: PatternMatcher::new(),
            working_dir,
        }
    }

    pub fn process(&self) {
        for pattern in &self.args.patterns {
            let path = Path::new(pattern);
            if path.exists() {
                if path.is_dir() {
                    self.process_directory(path);
                } else {
                    self.process_single_file(path);
                }
            } else {
                // Treat as a glob pattern
                self.process_glob_pattern(pattern);
            }
        }
    }

    fn process_glob_pattern(&self, pattern: &str) {
        let regex = self.pattern_matcher.glob_to_regex(pattern);
        let walker = self.create_walker();
        
        for entry in walker.into_iter().filter_entry(|e| self.should_process_entry(e.path())) {
            if let Ok(entry) = entry {
                let path = entry.path();
                if path.is_file() && regex.is_match(path.to_str().unwrap_or("")) {
                    self.process_single_file(path);
                }
            }
        }
    }

    fn process_directory(&self, dir: &Path) {
        let walker = WalkDir::new(dir).into_iter();
        for entry in walker.filter_entry(|e| self.should_process_entry(e.path())) {
            if let Ok(entry) = entry {
                let path = entry.path();
                if path.is_file() {
                    self.process_single_file(path);
                }
            }
        }
    }

    fn create_walker(&self) -> WalkDir {
        if self.args.recursive {
            WalkDir::new(&self.working_dir)
        } else {
            WalkDir::new(&self.working_dir).max_depth(1)
        }
    }

    fn should_process_entry(&self, path: &Path) -> bool {
        // First check if it's a .git directory or within one
        if path.components().any(|c| c.as_os_str() == ".git") {
            return false;
        }

        // Then check gitignore if enabled
        if let Some(gi) = &self.gitignore {
            !gi.matched(path, path.is_dir()).is_ignore()
        } else {
            true
        }
    }

    fn process_single_file(&self, path: &Path) {
        println!("# File: {}", path.display());
        match fs::read_to_string(path) {
            Ok(contents) => {
                println!("{}", contents);
                println!("\n=====================\n");
            }
            Err(_) => println!("Error reading file: {}", path.display()),
        }
    }
}
