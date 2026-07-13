use std::process::Command;
fn main() {
    let chrome_path = "/home/nhatanh/projet/alouette_studio/chrome/chrome";
    let profile_path = "/home/nhatanh/projet/alouette_studio/chrome_profile";
    let _ = std::fs::remove_dir_all(profile_path);
    let _ = std::fs::create_dir_all(profile_path);
    match Command::new(chrome_path)
        .arg(format!("--user-data-dir={}", profile_path))
        .arg("--no-first-run")
        .arg("--no-default-browser-check")
        .spawn() {
        Ok(mut child) => {
            println!("Spawned successfully with PID: {}", child.id());
            let _ = child.wait();
        },
        Err(e) => {
            println!("Failed to launch Google Chrome: {}", e);
        }
    }
}
