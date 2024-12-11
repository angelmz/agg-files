use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;
use chrono::{Local, DateTime};
use std::collections::HashSet;

use crate::cli::CliArgs;
use crate::ignore_files_helper::IgnoreFilesHelper;
use crate::pattern_matcher::PatternMatcher;
use crate::git_status_handler::GitHistoryHandler;

pub struct FileProcessor {
    args: CliArgs,
    ignore_helper: Option<IgnoreFilesHelper>,
    pattern_matcher: PatternMatcher,
    working_dir: PathBuf,
    files_to_process: Vec<PathBuf>,
    ignored_files: HashSet<PathBuf>,
    processed_files: HashSet<PathBuf>,
    output_dir: PathBuf,
    git_status_handler: Option<GitHistoryHandler>,
}

impl FileProcessor {
    pub fn new(args: CliArgs, working_dir: PathBuf) -> Self {
        let ignore_helper = if !args.ignore_gitignore && !args.ignore_custom {
            Some(IgnoreFilesHelper::new())
        } else {
            None
        };

        let git_status_handler = if args.git_changes {
            Some(GitHistoryHandler::new(working_dir.clone()))
        } else {
            None
        };

        // Set up output directory
        let output_dir = PathBuf::from("/Users/angel/agg-output");
        if !output_dir.exists() {
            if let Err(e) = fs::create_dir_all(&output_dir) {
                eprintln!("Warning: Could not create output directory: {}", e);
            }
        }

        Self {
            args,
            ignore_helper,
            pattern_matcher: PatternMatcher::new(),
            working_dir,
            files_to_process: Vec::new(),
            ignored_files: HashSet::new(),
            processed_files: HashSet::new(),
            output_dir,
            git_status_handler,
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

    fn collect_from_glob_pattern(&mut self, pattern: &str, files: &mut Vec<PathBuf>) {
        let regex = self.pattern_matcher.glob_to_regex(pattern);
        let walker = if self.args.recursive {
            WalkDir::new(&self.working_dir)
        } else {
            WalkDir::new(&self.working_dir).max_depth(1)
        };
        
        let should_process = |path: &Path| -> bool {
            !path.components().any(|c| c.as_os_str() == ".git") && 
            if let Some(ih) = &self.ignore_helper {
                !ih.is_ignored(path)
            } else {
                true
            }
        };
        
        for entry in walker.into_iter()
            .filter_entry(|e| should_process(e.path()))
            .filter_map(Result::ok)
            .filter(|e| e.path().is_file())
        {
            let path = entry.path();
            if regex.is_match(path.to_str().unwrap_or("")) && self.should_include_file(path) {
                self.processed_files.insert(path.to_path_buf());
                files.push(path.to_path_buf());
            } else {
                self.ignored_files.insert(path.to_path_buf());
            }
        }
    }

    fn collect_from_directory(&mut self, dir: &Path, files: &mut Vec<PathBuf>) {
        let walker = if self.args.recursive {
            WalkDir::new(dir)
        } else {
            WalkDir::new(dir).max_depth(1)
        };
        
        let should_process = |path: &Path| -> bool {
            !path.components().any(|c| c.as_os_str() == ".git") && 
            if let Some(ih) = &self.ignore_helper {
                !ih.is_ignored(path)
            } else {
                true
            }
        };
        
        for entry in walker.into_iter()
            .filter_entry(|e| should_process(e.path()))
            .filter_map(Result::ok)
            .filter(|e| e.path().is_file())
        {
            let path = entry.path();
            if self.should_include_file(path) {
                self.processed_files.insert(path.to_path_buf());
                files.push(path.to_path_buf());
            } else {
                self.ignored_files.insert(path.to_path_buf());
            }
        }
    }

    fn get_output_filename(&self, index: Option<usize>, total_chunks: Option<usize>, file_type: &str) -> PathBuf {
        let current_dir_name = self.working_dir
            .canonicalize()
            .map(|p| {
                p.file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("unknown_dir")
                    .to_owned()
            })
            .unwrap_or_else(|_| "unknown_dir".to_owned());

        let datetime = Local::now();
        let formatted_time = datetime.format("%Y%m%d_%H%M%S").to_string();
        
        let filename = if let Some(pattern) = &self.args.output_pattern {
            if let (Some(idx), Some(_)) = (index, total_chunks) {
                pattern.replace("{}", &(idx + 1).to_string())
            } else {
                pattern.replace("{}", "")
            }
        } else if let (Some(idx), Some(_)) = (index, total_chunks) {
            format!("{}_{}_part_{}.txt", current_dir_name, formatted_time, idx + 1)
        } else if file_type == "main" {
            format!("{}_{}.txt", current_dir_name, formatted_time)
        } else {
            format!("{}_{}_{}s.txt", current_dir_name, formatted_time, file_type)
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

    fn write_file_list(&self, filename: PathBuf, files: &HashSet<PathBuf>) -> std::io::Result<()> {
        let mut file = File::create(&filename)?;
        
        let working_dir = std::env::current_dir()?;
        writeln!(file, "Working Directory: {}\n", working_dir.display())?;
        
        let mut sorted_files: Vec<_> = files.iter().collect();
        sorted_files.sort();
        
        for path in sorted_files {
            let relative_path = if let Ok(rel_path) = path.strip_prefix(&working_dir) {
                rel_path.to_path_buf()
            } else {
                path.clone()
            };
            
            writeln!(file, "File: {}", relative_path.display())?;
        }
        
        println!("Created: {}", filename.display());
        Ok(())
    }

    fn process_with_git_history(&mut self) -> Vec<PathBuf> {
        if let Some(handler) = &self.git_status_handler {
            if !handler.is_git_repository() {
                eprintln!("Warning: Not a git repository. Skipping git history filtering.");
                return Vec::new();
            }
            
            let since_date = self.args.git_since.as_ref()
                .and_then(|date_str| DateTime::parse_from_rfc3339(date_str).ok());
                
            let changed_files = handler.get_changed_files(since_date);
            
            // Create a new Vec with only the changed files
            let git_changed_files: Vec<PathBuf> = self.files_to_process.iter()
                .filter(|file| changed_files.contains(*file))
                .cloned()
                .collect();
            
            // Create a separate output file for changed files
            let output_path = self.get_output_filename(None, None, "git_changes");
            
            match File::create(&output_path) {
                Ok(mut file) => {
                    let mut success_count = 0;
                    let mut total_size = 0;
                    
                    for path in &git_changed_files {
                        if let Ok(size) = fs::metadata(path).map(|m| m.len() as usize) {
                            total_size += size;
                        }
                        if self.process_file(path, &mut file).is_ok() {
                            success_count += 1;
                        }
                    }
                    
                    println!("Created git changes file: {} ({} files, size: {})",
                        output_path.display(),
                        success_count,
                        Self::format_size(total_size)
                    );
                }
                Err(e) => eprintln!("Error creating git changes file: {}", e),
            }
            
            git_changed_files
        } else {
            Vec::new()
        }
    }

    pub fn process(&mut self) {
        if self.files_to_process.is_empty() {
            self.collect_files();
        }
        
        if self.files_to_process.is_empty() {
            println!("No files found matching the patterns.");
            return;
        }
    
        // Store original files
        let original_files = self.files_to_process.clone();
    
        // Always try to create git changes file if git_changes is true
        if self.args.git_changes {
            let git_status_handler = GitHistoryHandler::new(self.working_dir.clone());
            if git_status_handler.is_git_repository() {
                self.git_status_handler = Some(git_status_handler);
                self.process_with_git_history();
            } else {
                println!("Note: Not a git repository - skipping git changes output");
            }
        }
    
        // Restore original files for normal processing
        self.files_to_process = original_files;
    
        // Process main output file
        let chunks = self.distribute_files();
        println!("\nSaving files to: {}", self.output_dir.display());
        
        for (i, chunk) in chunks.iter().enumerate() {
            if chunk.is_empty() {
                continue;
            }
            
            let output_path = self.get_output_filename(
                Some(i).filter(|_| chunks.len() > 1),
                Some(chunks.len()).filter(|_| chunks.len() > 1),
                "main"
            );
    
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
                    println!("Created {} ({} files, TOTAL size: {})", 
                        output_path.display(), 
                        success_count,
                        Self::format_size(chunk_size)
                    );
                }
                Err(e) => eprintln!("Error creating output file {}: {}", output_path.display(), e),
            }
        }
    
        // Optionally write files_read.txt
        if self.args.create_index {
            if let Err(e) = self.write_file_list(
                self.get_output_filename(None, None, "read"),
                &self.processed_files
            ) {
                eprintln!("Error writing read files list: {}", e);
            }
        }
        
        // Optionally write files_ignored.txt
        if !self.ignored_files.is_empty() && self.args.create_index {
            if let Err(e) = self.write_file_list(
                self.get_output_filename(None, None, "ignored"),
                &self.ignored_files
            ) {
                eprintln!("Error writing ignored files list: {}", e);
            }
        }
        
        println!("\nProcessing complete!");
    }
}