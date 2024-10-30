use std::env;

pub struct CliArgs {
    pub recursive: bool,
    pub ignore_gitignore: bool,
    pub patterns: Vec<String>,
    pub github_url: Option<String>,
    pub show_version: bool,
}

impl CliArgs {
    pub fn parse() -> Self {
        let args: Vec<String> = env::args().collect();
        let mut recursive = false;
        let mut ignore_gitignore = false;
        let mut patterns = Vec::new();
        let mut github_url = None;
        let mut show_version = false;
        let mut i = 1;

        while i < args.len() {
            match args[i].as_str() {
                "-r" => recursive = true,
                "-i" => ignore_gitignore = true,
                "-v" | "--version" => show_version = true,
                "--url" => {
                    if i + 1 < args.len() {
                        github_url = Some(args[i + 1].clone());
                        i += 1;
                    }
                }
                _ => {
                    if !args[i].starts_with('-') {
                        patterns.push(args[i].clone());
                    }
                }
            }
            i += 1;
        }

        // If no patterns specified and URL is provided, default to all files
        if patterns.is_empty() && github_url.is_some() {
            patterns.push("*".to_string());
        }

        Self {
            recursive,
            ignore_gitignore,
            patterns,
            github_url,
            show_version,
        }
    }

    pub fn is_valid(&self) -> bool {
        self.show_version || !self.patterns.is_empty() || self.github_url.is_some()
    }

    pub fn print_usage(&self) {
        let program_name = env::args().next().unwrap_or_else(|| String::from("program"));
        println!("Usage: {} [OPTIONS] [PATTERNS]", program_name);
        println!("\nOptions:");
        println!("  --url <github_url>  GitHub repository URL");
        println!("  -r                  Search recursively");
        println!("  -i                  Ignore .gitignore (include all files)");
        println!("  -v, --version       Show version information");
        println!("\nExamples:");
        println!("  {} --url 'https://github.com/org/repo/tree/main/path' -r", program_name);
        println!("  {} -r '*.{{rs,toml}}'", program_name);
        println!("  {} --version", program_name);
    }
}
