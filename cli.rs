use std::env;

pub struct CliArgs {
    pub recursive: bool,
    pub ignore_gitignore: bool,
    pub ignore_custom: bool,
    pub patterns: Vec<String>,
    pub github_url: Option<String>,
    pub show_version: bool,
    pub split_chunks: Option<usize>,
    pub output_pattern: Option<String>,
    pub create_index: bool,
    pub max_lines: Option<usize>,
}

impl CliArgs {
    pub fn parse() -> Self {
        let args: Vec<String> = env::args().collect();
        let mut recursive = false;
        let mut ignore_gitignore = false;
        let mut ignore_custom = false;
        let mut patterns = Vec::new();
        let mut github_url = None;
        let mut show_version = false;
        let mut split_chunks = None;
        let mut output_pattern = None;
        let mut create_index = false;
        let mut max_lines = None;
        let mut i = 1;

        while i < args.len() {
            match args[i].as_str() {
                "-r" => recursive = true,
                "-i" => ignore_gitignore = true,
                "--no-custom-ignore" => ignore_custom = true,
                "-v" | "--version" => show_version = true,
                "--index" => create_index = true,
                "--max-lines" => {
                    if i + 1 < args.len() {
                        if let Ok(n) = args[i + 1].parse::<usize>() {
                            max_lines = Some(n);
                        }
                        i += 1;
                    }
                }
                "-n" | "--chunks" => {
                    if i + 1 < args.len() {
                        if let Ok(n) = args[i + 1].parse::<usize>() {
                            split_chunks = Some(n);
                        }
                        i += 1;
                    }
                }
                "-o" | "--output" => {
                    if i + 1 < args.len() {
                        output_pattern = Some(args[i + 1].clone());
                        i += 1;
                    }
                }
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
            ignore_custom,
            patterns,
            github_url,
            show_version,
            split_chunks,
            output_pattern,
            create_index,
            max_lines,
        }
    }

    pub fn is_valid(&self) -> bool {
        self.show_version || !self.patterns.is_empty() || self.github_url.is_some()
    }

    pub fn print_usage(&self) {
        let program_name = env::args().next().unwrap_or_else(|| String::from("program"));
        println!("Usage: {} [OPTIONS] [PATTERNS]", program_name);
        println!("\nOptions:");
        println!("  --url <github_url>    GitHub repository URL");
        println!("  -r                    Search recursively");
        println!("  -i                    Ignore .gitignore (include all files)");
        println!("  --no-custom-ignore    Ignore the 'to_ignore' file");
        println!("  -v, --version         Show version information");
        println!("  -n, --chunks <N>      Split output into N files");
        println!("  -o, --output <pattern> Output file pattern (e.g., 'output.txt')");
        println!("  --index               Create additional files listing read and ignored files");
        println!("  --max-lines <N>       Skip files with more than N lines");
        println!("\nExamples:");
        println!("  {} -r --max-lines 1000 '*.rs'", program_name);
        println!("  {} -n 5 -o 'part_1.txt' '*.rs'", program_name);
        println!("  {} --index -r '**/*.rs'", program_name);
        println!("  {} --url 'https://github.com/username/repo' -r '*.rs'", program_name);
    }
}