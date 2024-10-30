use chrono::Utc;

fn main() {
    // Set the build date
    let build_date = Utc::now().format("%Y-%m-%d %H:%M:%S UTC").to_string();
    println!("cargo:rustc-env=BUILD_DATE={}", build_date);
    
    // Ensure the version information is always rebuilt
    println!("cargo:rerun-if-changed=build.rs");
}

