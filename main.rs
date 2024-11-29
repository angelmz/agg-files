mod cli;
mod file_processor;
mod gitignore_helper;
mod pattern_matcher;
mod github_handler;
mod temp_manager;
mod version;

use cli::CliArgs;
use file_processor::FileProcessor;
use github_handler::GitHubHandler;
use temp_manager::TempManager;
use std::path::PathBuf;
use version::Version;

#[tokio::main]
async fn main() {
    let args = CliArgs::parse();
    
    if args.show_version {
        Version::print();
        return;
    }

    if !args.is_valid() {
        args.print_usage();
        return;
    }

    let working_dir = if let Some(url) = &args.github_url {
        match process_github_url(url).await {
            Ok(dir) => dir,
            Err(e) => {
                eprintln!("Error processing GitHub URL: {}", e);
                return;
            }
        }
    } else {
        PathBuf::from(".")
    };

    let mut processor = FileProcessor::new(args, working_dir);  // Added mut here
    processor.process();
}

async fn process_github_url(url: &str) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let github_handler = GitHubHandler::new();
    let repo_info = github_handler.parse_url(url)?;
    
    let temp_manager = TempManager::new();
    let repo_path = temp_manager.get_repo_path(&repo_info);

    if !temp_manager.repo_exists(&repo_info) {
        github_handler.download_repository(&repo_info).await?;
    }

    Ok(repo_path)
}
