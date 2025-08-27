use std::fs;

fn main() {
    // Tell cargo to rerun this if .env changes
    println!("cargo:rerun-if-changed=.env");

    // Read the .env file
    let env_contents = fs::read_to_string(".env").expect("Failed to read .env file");

    // Parse each line and set as compile-time environment variable
    for line in env_contents.lines() {
        if line.starts_with('#') || line.is_empty() {
            continue;
        }

        if let Some((key, value)) = line.split_once('=') {
            let key = key.trim();
            let value = value
                .trim()
                .strip_prefix('"')
                .unwrap_or(value)
                .strip_suffix('"')
                .unwrap_or(value);
            // Set compile-time environment variable
            println!("cargo:rustc-env={}={}", key, value);
        }
    }
}
