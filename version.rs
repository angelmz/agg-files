use std::collections::HashMap;
use std::env::consts;

pub struct Version;

impl Version {
    pub fn print() {
        let version = env!("CARGO_PKG_VERSION");
        let name = env!("CARGO_PKG_NAME");
        let authors = env!("CARGO_PKG_AUTHORS");
        let description = env!("CARGO_PKG_DESCRIPTION");
        let repository = env!("CARGO_PKG_REPOSITORY");

        // Print package information
        println!("{} v{}", name, version);
        println!("Authors: {}", authors);
        println!("Description: {}", description);
        println!("Repository: {}", repository);

        // Print build information
        let build_info = Self::build_info();
        println!("\nBuild Information:");
        for (key, value) in build_info {
            println!("  {}: {}", key, value);
        }
    }

    fn build_info() -> HashMap<String, String> {
        let mut info = HashMap::new();
        
        // Add target information using std::env::consts
        info.insert(
            "Target".to_string(),
            format!("{}-{}", 
                consts::ARCH,
                consts::OS
            ),
        );

        // Add build profile
        #[cfg(debug_assertions)]
        info.insert("Profile".to_string(), "debug".to_string());
        #[cfg(not(debug_assertions))]
        info.insert("Profile".to_string(), "release".to_string());

        // Add build date (set during compilation)
        info.insert("Build Date".to_string(), env!("BUILD_DATE").to_string());

        // Add additional platform info
        info.insert("Family".to_string(), consts::FAMILY.to_string());

        info
    }
}
