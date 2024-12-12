use std::process::Command;
use std::path::PathBuf;
use std::collections::HashSet;
use chrono::{DateTime, FixedOffset};
use crate::ignore_files_helper::IgnoreFilesHelper;

pub struct GitChanges {
    pub modified_files: HashSet<PathBuf>,
    pub deleted_files: HashSet<PathBuf>,
    pub untracked_files: HashSet<PathBuf>,
}

pub struct GitStatusHandler {
    working_dir: PathBuf,
    ignore_helper: Option<IgnoreFilesHelper>,
}

impl GitStatusHandler {
    pub fn new(working_dir: PathBuf) -> Self {
        Self {
            working_dir,
            ignore_helper: Some(IgnoreFilesHelper::new()),
        }
    }

    fn is_staged(&self, file_path: &str) -> bool {
        let mut cmd = Command::new("git");
        cmd.current_dir(&self.working_dir);
        cmd.args(["diff", "--cached", "--name-only", file_path]);
        
        cmd.output()
            .map(|output| !output.stdout.is_empty())
            .unwrap_or(false)
    }

    fn should_process_file(&self, path: &PathBuf) -> bool {
        if let Some(ignore_helper) = &self.ignore_helper {
            !ignore_helper.is_ignored(path)
        } else {
            true
        }
    }

    pub fn get_changed_files(&self, since: Option<DateTime<FixedOffset>>) -> GitChanges {
        let mut modified_files = HashSet::new();
        let mut deleted_files = HashSet::new();
        let mut untracked_files = HashSet::new();
        
        // Get all status including untracked files
        let mut status_cmd = Command::new("git");
        status_cmd.current_dir(&self.working_dir);
        status_cmd.args(["status", "--porcelain", "-u", "--no-renames"]);
        
        if let Ok(output) = status_cmd.output() {
            if let Ok(files_str) = String::from_utf8(output.stdout) {
                for line in files_str.lines() {
                    if line.len() < 3 { continue; }
                    let status = &line[0..2];
                    let file_path = &line[3..];
                    let path = self.working_dir.join(file_path);

                    // Skip if the file is in the ignore list
                    if !self.should_process_file(&path) {
                        continue;
                    }
                    
                    match status {
                        " D" | "D " => {
                            if !self.is_staged(file_path) {
                                deleted_files.insert(path);
                            }
                        },
                        "??" => {
                            if path.exists() {
                                untracked_files.insert(path);
                            }
                        },
                        _ => {
                            if !self.is_staged(file_path) {
                                if path.exists() {
                                    modified_files.insert(path);
                                }
                            }
                        }
                    }
                }
            }
        }

        // Include committed files if since date is provided
        if let Some(date) = since {
            let mut log_cmd = Command::new("git");
            log_cmd.current_dir(&self.working_dir);
            log_cmd.args([
                "log",
                "--diff-filter=D",  // Only get deleted files
                "--name-status",    // Show status with filenames
                "--pretty=format:",
                &format!("--since={}", date.format("%Y-%m-%d"))
            ]);
            
            if let Ok(output) = log_cmd.output() {
                if let Ok(files_str) = String::from_utf8(output.stdout) {
                    for line in files_str.lines() {
                        if let Some(file_path) = line.strip_prefix('D') {
                            let path = self.working_dir.join(file_path.trim());
                            if self.should_process_file(&path) {
                                deleted_files.insert(path);
                            }
                        }
                    }
                }
            }
        }
        
        GitChanges {
            modified_files,
            deleted_files,
            untracked_files,
        }
    }

    pub fn is_git_repository(&self) -> bool {
        let mut cmd = Command::new("git");
        cmd.current_dir(&self.working_dir);
        cmd.args(["rev-parse", "--is-inside-work-tree"]);
        
        cmd.output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }
}