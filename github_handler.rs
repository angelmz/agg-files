use reqwest;
use std::error::Error;
use tokio::fs;
use flate2::read::GzDecoder;
use tar::Archive;
use url::Url;

pub struct RepoInfo {
    pub owner: String,
    pub repo: String,
    pub branch: String,
    pub path: Option<String>,
}

pub struct GitHubHandler {
    client: reqwest::Client,
}

impl GitHubHandler {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }

    pub fn parse_url(&self, url: &str) -> Result<RepoInfo, Box<dyn Error>> {
        let parsed_url = Url::parse(url)?;
        let path_segments: Vec<&str> = parsed_url.path_segments()
            .ok_or("Invalid URL")?
            .collect();

        if path_segments.len() < 2 {
            return Err("Invalid GitHub URL".into());
        }

        let owner = path_segments[0].to_string();
        let repo = path_segments[1].to_string();
        
        let (branch, path) = if path_segments.len() > 3 && path_segments[2] == "tree" {
            let branch = path_segments[3].to_string();
            let path = if path_segments.len() > 4 {
                Some(path_segments[4..].join("/"))
            } else {
                None
            };
            (branch, path)
        } else {
            ("main".to_string(), None)
        };

        Ok(RepoInfo {
            owner,
            repo,
            branch,
            path,
        })
    }

    pub async fn download_repository(&self, repo_info: &RepoInfo) -> Result<(), Box<dyn Error>> {
        let temp_manager = crate::temp_manager::TempManager::new();
        let target_dir = temp_manager.get_repo_path(repo_info);

        // Create target directory if it doesn't exist
        fs::create_dir_all(&target_dir).await?;

        // Download tarball
        let url = format!(
            "https://api.github.com/repos/{}/{}/tarball/{}",
            repo_info.owner, repo_info.repo, repo_info.branch
        );

        let response = self.client
            .get(&url)
            .header("User-Agent", "rust-file-finder")
            .send()
            .await?;

        let bytes = response.bytes().await?;
        
        // Extract tarball
        let decoder = GzDecoder::new(&bytes[..]);
        let mut archive = Archive::new(decoder);
        
        // Use a temporary directory for extraction
        let temp_dir = target_dir.join("temp");
        fs::create_dir_all(&temp_dir).await?;
        
        // Extract files
        archive.unpack(&temp_dir)?;

        // Move files from the extracted directory to the target directory
        let extracted_dir = std::fs::read_dir(&temp_dir)?
            .next()
            .ok_or("No files extracted")??.path();

        if let Some(path) = &repo_info.path {
            let source_dir = extracted_dir.join(path);
            if source_dir.exists() {
                std::fs::rename(source_dir, &target_dir)?;
            } else {
                return Err(format!("Path '{}' not found in repository", path).into());
            }
        } else {
            // Move all files from extracted directory to target directory
            for entry in std::fs::read_dir(extracted_dir)? {
                let entry = entry?;
                let target_path = target_dir.join(entry.file_name());
                std::fs::rename(entry.path(), target_path)?;
            }
        }

        // Clean up temporary directory
        std::fs::remove_dir_all(temp_dir)?;

        Ok(())
    }
}
