use std::{
    env, fs,
    process::{Command, Stdio},
};

const DEFAULT_DOWNLOAD_URL: &str = "https://elyra.example.com/downloads/Elyra.msi";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let url = env::var("ELYRA_DOWNLOAD_URL").unwrap_or_else(|_| DEFAULT_DOWNLOAD_URL.to_string());
    let directory = env::temp_dir().join("elyra-installer");
    fs::create_dir_all(&directory)?;
    let installer = directory.join("Elyra-latest.msi");
    let response = reqwest::blocking::get(&url)?.error_for_status()?;
    fs::write(&installer, response.bytes()?)?;
    Command::new("msiexec")
        .args(["/i", installer.to_string_lossy().as_ref(), "/passive"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    Ok(())
}
