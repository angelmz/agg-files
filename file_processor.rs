use std::fs::{self, File, create_dir_all, read_dir};
use std::io::Write;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;
use chrono::{DateTime, Local};

use crate::cli::CliArgs;
use crate::gitignore_helper::GitignoreHelper;
use crate::pattern_matcher::PatternMatcher;

pub struct FileProcessor {
    args: CliArgs,
    gitignore: Option<ignore::gitignore::Gitignore>,
    pattern_matcher: PatternMatcher,
    working_dir: PathBuf,
    files_to_process: Vec<PathBuf>,
    output_dir: PathBuf,
}

impl FileProcessor {
    pub fn new(args: CliArgs, working_dir: PathBuf) -> Self {
        let gitignore = if !args.ignore_gitignore {
            GitignoreHelper::build()
        } else {
            None
        };

        // Get the canonical path and extract the directory name
        let current_dir_name = working_dir
            .canonicalize()
            .map(|p| {
                p.file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("unknown_dir")
                    .to_owned()
            })
            .unwrap_or_else(|_| "unknown_dir".to_owned());

        let datetime: DateTime<Local> = Local::now();
        let formatted_time = datetime.format("%Y%m%d_%H%M%S").to_string();

        // Create the output directory path with timestamp
        let output_dir = PathBuf::from("/Users/villanelle/agg-output")
            .join(format!("{}_{}", current_dir_name, formatted_time));

        Self {
            args,
            gitignore,
            pattern_matcher: PatternMatcher::new(),
            working_dir,
            files_to_process: Vec::new(),
            output_dir,
        }
    }

    fn is_binary_file(path: &Path) -> bool {
        if let Ok(metadata) = std::fs::metadata(path) {
            // Skip if file is too large (> 1MB) to avoid memory issues
            if metadata.len() > 1_000_000 {
                return true;
            }

            if let Ok(content) = std::fs::read(path) {
                // Check first 1024 bytes for null bytes
                let check_size = std::cmp::min(1024, content.len());
                let contains_null = content[..check_size].contains(&0);
                return contains_null;
            }
        }
        false
    }

    fn count_lines(path: &Path) -> std::io::Result<usize> {
        // First check if it's a binary file
        if Self::is_binary_file(path) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Binary file detected"
            ));
        }
        
        let content = fs::read_to_string(path)?;
        Ok(content.lines().count())
    }

    fn should_include_file(&self, path: &Path) -> bool {
        if let Some(max_lines) = self.args.max_lines {
            match Self::count_lines(path) {
                Ok(line_count) => {
                    if line_count > max_lines {
                        println!("Skipping {} (has {} lines, max is {})", 
                            path.display(), line_count, max_lines);
                        return false;
                    }
                    true
                },
                Err(e) if e.kind() == std::io::ErrorKind::InvalidData => {
                    // Binary file detected, skip silently
                    false
                },
                Err(e) => {
                    eprintln!("Error counting lines in {}: {}", path.display(), e);
                    false
                }
            }
        } else {
            true
        }
    }

    fn clean_output_directory(&self) -> std::io::Result<()> {
        if self.output_dir.exists() {
            println!("Cleaning output directory: {}", self.output_dir.display());
            for entry in read_dir(&self.output_dir)? {
                if let Ok(entry) = entry {
                    let path = entry.path();
                    if path.is_file() {
                        if let Err(e) = fs::remove_file(&path) {
                            eprintln!("Warning: Could not remove file {}: {}", path.display(), e);
                        }
                    }
                }
            }
            println!("Output directory cleaned.");
        }
        create_dir_all(&self.output_dir)?;
        Ok(())
    }

    fn collect_files(&mut self) {
        let mut files = Vec::new();
        let patterns = self.args.patterns.clone();

        for pattern in patterns {
            let path = Path::new(&pattern);
            if path.exists() {
                if path.is_dir() {
                    self.collect_from_directory(path, &mut files);
                } else {
                    files.push(path.to_path_buf());
                }
            } else {
                self.collect_from_glob_pattern(&pattern, &mut files);
            }
        }

        files.sort();
        files.dedup();
        self.files_to_process = files;
    }

    fn collect_from_glob_pattern(&self, pattern: &str, files: &mut Vec<PathBuf>) {
        let regex = self.pattern_matcher.glob_to_regex(pattern);
        let walker = self.create_walker();
        
        for entry in walker.into_iter().filter_entry(|e| self.should_process_entry(e.path())) {
            if let Ok(entry) = entry {
                let path = entry.path();
                if path.is_file() && 
                   regex.is_match(path.to_str().unwrap_or("")) && 
                   self.should_include_file(path) {
                    files.push(path.to_path_buf());
                }
            }
        }
    }

    fn collect_from_directory(&self, dir: &Path, files: &mut Vec<PathBuf>) {
        let walker = WalkDir::new(dir).into_iter();
        for entry in walker.filter_entry(|e| self.should_process_entry(e.path())) {
            if let Ok(entry) = entry {
                let path = entry.path();
                if path.is_file() && self.should_include_file(path) {
                    files.push(path.to_path_buf());
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
        if path.components().any(|c| c.as_os_str() == ".git") {
            return false;
        }

        if let Some(gi) = &self.gitignore {
            !gi.matched(path, path.is_dir()).is_ignore()
        } else {
            true
        }
    }

    fn get_output_filename(&self, index: usize, total_chunks: usize) -> PathBuf {
        let filename = if let Some(pattern) = &self.args.output_pattern {
            if total_chunks > 1 {
                pattern.replace("{}", &(index + 1).to_string())
            } else {
                pattern.replace("{}", "")
            }
        } else if total_chunks > 1 {
            format!("output_{}.txt", index + 1)
        } else {
            "output.txt".to_string()
        };

        self.output_dir.join(filename)
    }

    fn format_size(size: usize) -> String {
        const KB: usize = 1024;
        const MB: usize = KB * 1024;
        const GB: usize = MB * 1024;

        if size >= GB {
            format!("{:.2} GB", size as f64 / GB as f64)
        } else if size >= MB {
            format!("{:.2} MB", size as f64 / MB as f64)
        } else if size >= KB {
            format!("{:.2} KB", size as f64 / KB as f64)
        } else {
            format!("{} B", size)
        }
    }

    fn distribute_files(&self) -> Vec<Vec<PathBuf>> {
        if let Some(chunks) = self.args.split_chunks {
            if chunks == 0 || self.files_to_process.is_empty() {
                return vec![self.files_to_process.clone()];
            }

            let mut files_with_sizes: Vec<(PathBuf, usize)> = Vec::new();

            for file in &self.files_to_process {
                if let Ok(size) = fs::metadata(file).map(|m| m.len() as usize) {
                    files_with_sizes.push((file.clone(), size));
                }
            }

            files_with_sizes.sort_by(|a, b| b.1.cmp(&a.1));

            let mut result = vec![Vec::new(); chunks];
            let mut chunk_sizes = vec![0; chunks];

            for (file, size) in files_with_sizes {
                let smallest_chunk_index = chunk_sizes
                    .iter()
                    .enumerate()
                    .min_by_key(|(_i, &size)| size)
                    .map(|(i, _)| i)
                    .unwrap_or(0);

                result[smallest_chunk_index].push(file);
                chunk_sizes[smallest_chunk_index] += size;
            }

            result.retain(|chunk| !chunk.is_empty());

            println!("\nFile distribution summary:");
            for (i, (chunk, &size)) in result.iter().zip(chunk_sizes.iter()).enumerate() {
                println!("Chunk {} size: {} ({} files)", 
                    i + 1,
                    Self::format_size(size),
                    chunk.len()
                );
            }

            if result.len() < chunks {
                println!("\nNote: Created {} chunks instead of the requested {} due to the number of files available.",
                    result.len(), chunks);
            }

            result
        } else {
            vec![self.files_to_process.clone()]
        }
    }

    fn process_file(&self, file: &Path, output: &mut impl Write) -> std::io::Result<()> {
        writeln!(output, "# File: {}", file.display())?;
        let contents = fs::read_to_string(file)?;
        writeln!(output, "{}", contents)?;
        writeln!(output, "\n=====================\n")?;
        Ok(())
    }

    fn create_index_file(&self) -> std::io::Result<()> {
        let index_path = self.output_dir.join("file_index.txt");
        let mut index_file = File::create(&index_path)?;
        
        let working_dir = std::env::current_dir()?;
        
        writeln!(index_file, "File Index")?;
        writeln!(index_file, "==========\n")?;
        writeln!(index_file, "Working Directory: {}", working_dir.display())?;
        if let Some(max_lines) = self.args.max_lines {
            writeln!(index_file, "Maximum Lines Per File: {}\n", max_lines)?;
        }
        
        let mut sorted_files = self.files_to_process.clone();
        sorted_files.sort();
        
        for file in sorted_files {
            let display_path = if let Ok(rel_path) = file.strip_prefix(&working_dir) {
                rel_path.to_path_buf()
            } else {
                file.clone()
            };
            
            if let Some(parent) = display_path.parent() {
                writeln!(index_file, "Directory: {}", parent.display())?;
                writeln!(index_file, "File: {}", display_path.file_name().unwrap_or_default().to_string_lossy())?;
            } else {
                writeln!(index_file, "File: {}", display_path.display())?;
            }
            
            if let Ok(metadata) = file.metadata() {
                writeln!(index_file, "Size: {}", Self::format_size(metadata.len() as usize))?;
                if let Ok(line_count) = Self::count_lines(&file) {
                    writeln!(index_file, "Lines: {}", line_count)?;
                }
                if let Ok(modified) = metadata.modified() {
                    if let Ok(modified_time) = modified.duration_since(std::time::UNIX_EPOCH) {
                        writeln!(index_file, "Last Modified: {} seconds since epoch", modified_time.as_secs())?;
                    }
                }
            }
            writeln!(index_file, "---")?;
        }
        
        println!("Created index file: {}", index_path.display());
        Ok(())
    }

    pub fn process(&mut self) {
        // Handle directory cleaning if requested
        if self.args.clean_output {
            if let Err(e) = self.clean_output_directory() {
                eprintln!("Error cleaning output directory: {}", e);
                return;
            }
        } else {
            // Just ensure the output directory exists
            if let Err(e) = create_dir_all(&self.output_dir) {
                eprintln!("Error creating output directory: {}", e);
                return;
            }
        }

        // First collect all files
        self.collect_files();
        
        if self.files_to_process.is_empty() {
            println!("No files found matching the patterns.");
            return;
        }

        // Create index file if requested
        if self.args.create_index {
            if let Err(e) = self.create_index_file() {
                eprintln!("Error creating index file: {}", e);
            }
        }
        
        // Then distribute and process them
        let chunks = self.distribute_files();
        
        println!("\nSaving files to: {}", self.output_dir.display());
        
        for (i, chunk) in chunks.iter().enumerate() {
            if chunk.is_empty() {
                continue;
            }
            
            let output_path = self.get_output_filename(i, chunks.len());
            
            match File::create(&output_path) {
                Ok(mut file) => {
                    let mut success_count = 0;
                    let mut chunk_size = 0;
                    
                    for path in chunk {
                        if let Ok(size) = fs::metadata(path).map(|m| m.len() as usize) {
                            chunk_size += size;
                        }
                        if self.process_file(path, &mut file).is_ok() {
                            success_count += 1;
                        }
                    }
                    println!("Created {} ({} files, total size: {})", 
                        output_path.display(), 
                        success_count,
                        Self::format_size(chunk_size)
                    );
                }
                Err(e) => eprintln!("Error creating output file {}: {}", output_path.display(), e),
            }
        }
        
        println!("\nProcessing complete. All files saved to: {}", self.output_dir.display());
    }
}