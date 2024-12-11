use std::process::Command;
use std::path::PathBuf;
use std::collections::HashSet;
use chrono::{DateTime, FixedOffset};

pub struct GitStatusHandler {
    working_dir: PathBuf,
}

impl GitStatusHandler {
    pub fn new(working_dir: PathBuf) -> Self {
        Self { working_dir }
    }

    fn is_staged(&self, file_path: &str) -> bool {
        let mut cmd = Command::new("git");
        cmd.current_dir(&self.working_dir);
        cmd.args(["diff", "--cached", "--name-only", file_path]);
        
        cmd.output()
            .map(|output| !output.stdout.is_empty())
            .unwrap_or(false)
    }

    pub fn get_changed_files(&self, since: Option<DateTime<FixedOffset>>) -> HashSet<PathBuf> {
        let mut changed_files = HashSet::new();
        
        // Get modified files from git status
        let mut status_cmd = Command::new("git");
        status_cmd.current_dir(&self.working_dir);
        status_cmd.args(["status", "--porcelain"]);
        
        if let Ok(output) = status_cmd.output() {
            if let Ok(files_str) = String::from_utf8(output.stdout) {
                for line in files_str.lines() {
                    if let Some(file_path) = line.get(3..) {  // Skip the status codes
                        if !self.is_staged(file_path) {  // Only include if not staged
                            let path = self.working_dir.join(file_path);
                            if path.exists() {
                                changed_files.insert(path);
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
                "--name-only",
                "--pretty=format:",
                &format!("--since={}", date.format("%Y-%m-%d"))
            ]);
            
            if let Ok(output) = log_cmd.output() {
                if let Ok(files_str) = String::from_utf8(output.stdout) {
                    for file in files_str.lines() {
                        if !file.is_empty() {
                            let file_path = self.working_dir.join(file);
                            if file_path.exists() {
                                changed_files.insert(file_path);
                            }
                        }
                    }
                }
            }
        }
        
        changed_files
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