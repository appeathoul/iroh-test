use std::env;
use std::fs;
use std::path::Path;

fn main() {
    // Get output directory, directly use target/debug or target/release
    let profile = env::var("PROFILE").unwrap_or_else(|_| "debug".to_string());
    let images_target_dir = Path::new("target").join(&profile).join("images");

    // Source images directory
    let source_images = Path::new("images");

    if source_images.exists() {
        // Create target directory
        if let Err(e) = fs::create_dir_all(&images_target_dir) {
            println!("cargo:warning=Failed to create images directory: {}", e);
            return;
        }

        // Copy all image files
        if let Ok(entries) = fs::read_dir(source_images) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() && !path.file_name().unwrap().to_string_lossy().starts_with('.') {
                    let file_name = path.file_name().unwrap();
                    let target_file = images_target_dir.join(file_name);

                    if let Err(e) = fs::copy(&path, &target_file) {
                        println!("cargo:warning=Failed to copy {}: {}", path.display(), e);
                    } else {
                        println!(
                            "cargo:warning=Copied {} to {}",
                            path.display(),
                            target_file.display()
                        );
                    }
                }
            }
        }
    } else {
        println!(
            "cargo:warning=Images directory not found at {}",
            source_images.display()
        );
    }

    // Tell Cargo to rerun when images directory changes
    println!("cargo:rerun-if-changed=images");
}
