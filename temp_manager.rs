use std::path::PathBuf;
use directories::ProjectDirs;
use std::fs;
use crate::github_handler::RepoInfo;

pub struct TempManager {
    base_dir: PathBuf,
}

impl TempManager {
    pub fn new() -> Self {
        let project_dirs = ProjectDirs::from("com", "seth4242", "agg-files")
            .expect("Failed to get project directories");
        
        let base_dir = project_dirs.cache_dir().to_path_buf();
        
        // Create base directory if it doesn't exist
        fs::create_dir_all(&base_dir).unwrap_or_else(|_| {
            eprintln!("Warning: Failed to create cache directory");
        });

        Self { base_dir }
    }

    pub fn get_repo_path(&self, repo_info: &RepoInfo) -> PathBuf {
        let repo_dir = self.base_dir
            .join(&repo_info.owner)
            .join(&repo_info.repo)
            .join(&repo_info.branch);

        if let Some(path) = &repo_info.path {
            repo_dir.join(path)
        } else {
            repo_dir
        }
    }

    pub fn repo_exists(&self, repo_info: &RepoInfo) -> bool {
        self.get_repo_path(repo_info).exists()
    }
}
