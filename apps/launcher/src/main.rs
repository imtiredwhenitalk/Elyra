//! Elyra launcher: download, verify and install the latest Windows build.
//!
//! Running with no arguments downloads the newest MSI from GitHub Releases,
//! verifies it, installs it quietly and opens Elyra. Explicit commands are
//! available for scripted or support use.

use sha2::{Digest, Sha256};
use std::{
    env, fmt::Write as _, fs,
    io::{self, Read, Write},
    path::{Path, PathBuf},
    process::{Command, ExitCode, Stdio},
    time::Duration,
};
use reqwest::blocking::Client;

const DEFAULT_DOWNLOAD_URL: &str =
    "https://github.com/imtiredwhenitalk/Elyra/releases/latest/download/Elyra-latest-x64.msi";
const MSI_MAGIC: [u8; 8] = [0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1];
const APP_NAME: &str = "Elyra";
const VERSION: &str = env!("CARGO_PKG_VERSION");

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CommandKind {
    Install,
    Uninstall,
    Launch,
    Info,
    Help,
}

struct Options {
    command: CommandKind,
    msi: Option<PathBuf>,
    url: String,
    sha256: Option<String>,
    silent: bool,
    open_after_install: bool,
}

impl Options {
    fn parse(args: &[String]) -> Result<Self> {
        let mut options = Self {
            command: CommandKind::Install,
            msi: None,
            url: env::var("ELYRA_DOWNLOAD_URL").unwrap_or_else(|_| DEFAULT_DOWNLOAD_URL.to_string()),
            sha256: env::var("ELYRA_MSI_SHA256").ok(),
            silent: false,
            open_after_install: true,
        };

        let mut index = 0;
        while index < args.len() {
            let argument = args[index].as_str();
            match argument {
                "install" | "update" => options.command = CommandKind::Install,
                "uninstall" | "remove" => options.command = CommandKind::Uninstall,
                "launch" | "open" => options.command = CommandKind::Launch,
                "info" | "status" => options.command = CommandKind::Info,
                "help" | "--help" | "-h" => options.command = CommandKind::Help,
                "--version" | "-V" => {
                    println!("Elyra Launcher {VERSION}");
                    std::process::exit(0);
                }
                "--silent" | "--quiet" | "-q" => options.silent = true,
                "--no-launch" => options.open_after_install = false,
                "--msi" | "--file" => {
                    index += 1;
                    let value = args
                        .get(index)
                        .ok_or("--msi needs a path to an MSI file")?;
                    options.msi = Some(PathBuf::from(value));
                }
                "--url" => {
                    index += 1;
                    options.url = args
                        .get(index)
                        .ok_or("--url needs a download URL")?
                        .clone();
                }
                "--sha256" | "--checksum" => {
                    index += 1;
                    let value = args
                        .get(index)
                        .ok_or("--sha256 needs a 64-character hex digest")?;
                    options.sha256 = Some(normalize_hash(value)?);
                }
                other if other.starts_with('-') => {
                    return Err(format!("unknown option: {other}").into());
                }
                other => {
                    return Err(format!("unknown command: {other}").into());
                }
            }
            index += 1;
        }

        if let Some(hash) = options.sha256.take() {
            options.sha256 = Some(normalize_hash(&hash)?);
        }

        Ok(options)
    }
}

fn main() -> ExitCode {
    let arguments: Vec<String> = env::args().skip(1).collect();
    let interactive = arguments.is_empty();

    match run(&arguments) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("\nElyra launcher: {error}");
            if interactive {
                eprintln!("Press Enter to close.");
                let mut line = String::new();
                let _ = io::stdin().read_line(&mut line);
            }
            ExitCode::FAILURE
        }
    }
}

fn run(arguments: &[String]) -> Result<()> {
    let options = Options::parse(arguments)?;

    match options.command {
        CommandKind::Help => {
            print_help();
            Ok(())
        }
        CommandKind::Info => {
            print_info();
            Ok(())
        }
        CommandKind::Uninstall => uninstall(options.silent),
        CommandKind::Launch => launch_app(),
        CommandKind::Install => install(&options),
    }
}

fn install(options: &Options) -> Result<()> {
    header("Install Elyra");

    let (msi_path, downloaded) = match &options.msi {
        Some(path) => {
            let path = path.canonicalize().unwrap_or_else(|_| path.clone());
            if !path.is_file() {
                return Err(format!("MSI not found: {}", path.display()).into());
            }
            println!("Using local package: {}", path.display());
            (path, false)
        }
        None => {
            let client = http_client()?;
            let directory = download_directory();
            fs::create_dir_all(&directory)?;
            let destination = directory.join("Elyra-latest-x64.msi");
            println!("Downloading {}", options.url);
            let actual = download(&client, &options.url, &destination)?;
            (destination, true)
        }
    };

    validate_package(&msi_path)?;
    let actual_hash = sha256_file(&msi_path)?;

    let expected_hash = match &options.sha256 {
        Some(hash) => Some(hash.clone()),
        None if downloaded => {
            let client = http_client()?;
            fetch_sidecar_hash(&client, &options.url)
        }
        None => None,
    };

    match expected_hash {
        Some(expected) => {
            if actual_hash != expected {
                if downloaded {
                    let _ = fs::remove_file(&msi_path);
                }
                return Err(format!(
                    "checksum mismatch\n  expected {expected}\n  actual   {actual_hash}"
                )
                .into());
            }
            println!("SHA-256 verified: {actual_hash}");
        }
        None => {
            println!("SHA-256: {actual_hash}");
            println!("No published checksum was found; the MSI signature and Windows Installer still apply.");
        }
    }

    install_package(&msi_path, options.silent)?;

    println!("\nElyra is installed.");
    if options.open_after_install {
        match launch_app() {
            Ok(()) => {}
            Err(error) => eprintln!("Could not open Elyra automatically: {error}"),
        }
    }
    Ok(())
}

fn http_client() -> Result<Client> {
    Ok(Client::builder()
        .user_agent(format!("Elyra-Launcher/{VERSION}"))
        .connect_timeout(Duration::from_secs(30))
        .timeout(Duration::from_secs(900))
        .build()?)
}

fn download_directory() -> PathBuf {
    env::temp_dir().join("elyra-installer")
}

fn download(client: &Client, url: &str, destination: &Path) -> Result<String> {
    let mut response = client.get(url).send()?.error_for_status()?;
    let total = response.content_length();
    let mut file = fs::File::create(destination)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    let mut downloaded: u64 = 0;
    let mut last_report: u64 = 0;

    loop {
        let read = response.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        file.write_all(&buffer[..read])?;
        hasher.update(&buffer[..read]);
        downloaded += read as u64;

        if downloaded - last_report >= 512 * 1024 {
            report_progress(downloaded, total);
            last_report = downloaded;
        }
    }
    report_progress(downloaded, total);
    println!();

    if downloaded == 0 {
        return Err("the download was empty".into());
    }

    Ok(to_hex(&hasher.finalize()))
}

fn report_progress(downloaded: u64, total: Option<u64>) {
    let downloaded_mb = downloaded as f64 / 1_048_576.0;
    match total {
        Some(total) if total > 0 => {
            let percent = (downloaded as f64 / total as f64 * 100.0).min(100.0);
            print!("\r  {percent:5.1}%  {downloaded_mb:6.1} MB");
        }
        _ => print!("\r  {downloaded_mb:6.1} MB"),
    }
    let _ = io::stdout().flush();
}

fn fetch_sidecar_hash(client: &Client, url: &str) -> Option<String> {
    let checksum_url = format!("{url}.sha256");
    let response = client.get(&checksum_url).send().ok()?;
    if !response.status().is_success() {
        return None;
    }
    let text = response.text().ok()?;
    let first = text.split_whitespace().next()?;
    normalize_hash(first).ok()
}

fn normalize_hash(value: &str) -> Result<String> {
    let hash = value
        .trim()
        .trim_start_matches("sha256:")
        .trim_start_matches("SHA256:")
        .to_ascii_lowercase();
    if hash.len() != 64 || !hash.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err("a SHA-256 digest must contain exactly 64 hex characters".into());
    }
    Ok(hash)
}

fn sha256_file(path: &Path) -> Result<String> {
    let mut file = fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(to_hex(&hasher.finalize()))
}

fn to_hex(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        let _ = write!(output, "{byte:02x}");
    }
    output
}

fn validate_package(path: &Path) -> Result<()> {
    let metadata = fs::metadata(path)?;
    if metadata.len() < 512 {
        return Err("the package is too small to be an MSI file".into());
    }

    let mut header = [0_u8; 8];
    let mut file = fs::File::open(path)?;
    file.read_exact(&mut header)?;
    if header != MSI_MAGIC {
        return Err("the downloaded file is not a Windows Installer package".into());
    }

    println!(
        "Package ready: {} ({:.1} MB)",
        path.display(),
        metadata.len() as f64 / 1_048_576.0
    );
    Ok(())
}

fn install_package(path: &Path, silent: bool) -> Result<()> {
    let mode = if silent { "/quiet" } else { "/passive" };
    println!("Installing with Windows Installer ({mode})…");

    let status = Command::new("msiexec")
        .arg("/i")
        .arg(path)
        .arg(mode)
        .arg("/norestart")
        .stdin(Stdio::null())
        .status()?;

    if status.success() {
        return Ok(());
    }

    // 3010 means the install succeeded and Windows wants a reboot.
    if status.code() == Some(3010) {
        println!("The installer finished; Windows will complete the update after a restart.");
        return Ok(());
    }

    Err(format!("Windows Installer exited with {status}").into())
}

fn uninstall(silent: bool) -> Result<()> {
    header("Remove Elyra");

    let record = find_install();
    let Some(record) = record else {
        println!("Elyra is not registered for this user. Nothing to remove.");
        return Ok(());
    };

    if let Some(version) = &record.display_version {
        println!("Found Elyra {version}");
    }

    if let Some(product_code) = &record.product_code {
        let mode = if silent { "/quiet" } else { "/passive" };
        let status = Command::new("msiexec")
            .args(["/x", product_code.as_str(), mode, "/norestart"])
            .stdin(Stdio::null())
            .status()?;
        if status.success() || status.code() == Some(3010) {
            println!("Elyra was removed. Your local mailbox database was left in place.");
            return Ok(());
        }
        return Err(format!("Windows Installer exited with {status}").into());
    }

    if let Some(uninstall_string) = &record.uninstall_string {
        let status = Command::new("cmd")
            .args(["/C", uninstall_string.as_str()])
            .stdin(Stdio::null())
            .status()?;
        if status.success() {
            println!("Elyra was removed.");
            return Ok(());
        }
        return Err(format!("The uninstaller exited with {status}").into());
    }

    Err("Elyra's uninstall entry has no product code or uninstall command".into())
}

fn print_help() {
    println!(
        "Elyra Launcher {VERSION}

USAGE
  elyra-launcher                 Download, verify, install and open Elyra
  elyra-launcher install         Same as running with no arguments
  elyra-launcher uninstall       Remove Elyra, keeping the local mailbox
  elyra-launcher launch          Open an installed Elyra
  elyra-launcher info            Show launcher and install details

OPTIONS
  --msi <path>                   Install a local MSI instead of downloading
  --url <url>                    Use a custom MSI URL
  --sha256 <digest>              Require a specific SHA-256 checksum
  --silent                       Fully quiet installer (default is /passive)
  --no-launch                    Do not open Elyra after installing
  --version                      Print the launcher version
  -h, --help                     Show this help

ENVIRONMENT
  ELYRA_DOWNLOAD_URL             Default MSI URL
  ELYRA_MSI_SHA256               Expected SHA-256 checksum

The default package is downloaded from GitHub Releases. A sibling
<url>.sha256 file is used automatically when it is published."
    );
}

fn print_info() {
    header("Elyra Launcher");
    println!("Launcher version : {VERSION}");
    println!("Platform         : {} {}", env::consts::OS, env::consts::ARCH);
    println!("Download URL    : {}", DEFAULT_DOWNLOAD_URL);
    println!("Download folder : {}", download_directory().display());

    match find_app_executable() {
        Some(path) => println!("Application     : {}", path.display()),
        None => println!("Application     : not found"),
    }

    match find_install() {
        Some(record) => {
            println!("Installed       : yes");
            if let Some(version) = record.display_version {
                println!("Installed version: {version}");
            }
            if let Some(location) = record.install_location {
                println!("Install location : {}", location.display());
            }
        }
        None => println!("Installed       : not registered"),
    }
}

fn header(title: &str) {
    println!("{title}");
    println!("{}", "─".repeat(title.chars().count()));
}

fn launch_app() -> Result<()> {
    let executable = find_app_executable().ok_or(
        "Elyra is not installed yet. Run the launcher without arguments to install it.",
    )?;

    println!("Opening {}", executable.display());
    Command::new(&executable)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    Ok(())
}

fn app_candidates() -> Vec<PathBuf> {
    let mut candidates = Vec::new();

    for variable in ["LOCALAPPDATA", "PROGRAMFILES", "PROGRAMFILES(X86)"] {
        let Ok(base) = env::var(variable) else {
            continue;
        };
        for folder in ["Elyra", "elyra", "Programs/Elyra"] {
            for executable in ["Elyra.exe", "elyra.exe", "Elyra Launcher.exe"] {
                candidates.push(PathBuf::from(&base).join(folder).join(executable));
            }
        }
    }

    if let Some(record) = find_install() {
        if let Some(location) = record.install_location {
            candidates.push(location.join("Elyra.exe"));
            candidates.push(location.join("elyra.exe"));
        }
    }

    candidates
}

fn find_app_executable() -> Option<PathBuf> {
    app_candidates().into_iter().find(|path| path.is_file())
}

struct InstallRecord {
    product_code: Option<String>,
    uninstall_string: Option<String>,
    install_location: Option<PathBuf>,
    display_version: Option<String>,
}

#[cfg(windows)]
fn find_install() -> Option<InstallRecord> {
    use winreg::{enums::*, RegKey};

    let roots = [HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE];
    let bases = [
        r"Software\Microsoft\Windows\CurrentVersion\Uninstall",
        r"Software\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall",
    ];

    for root in roots {
        for base in bases {
            let Ok(uninstall_root) = RegKey::predef(root).open_subkey(base) else {
                continue;
            };

            for name in uninstall_root.enum_keys().flatten() {
                let Ok(entry) = uninstall_root.open_subkey(&name) else {
                    continue;
                };
                let display_name: String = entry.get_value("DisplayName").unwrap_or_default();
                if !display_name.eq_ignore_ascii_case(APP_NAME)
                    && !display_name.starts_with(&format!("{APP_NAME} "))
                {
                    continue;
                }

                let uninstall_string: Option<String> = entry.get_value("UninstallString").ok();
                let product_code = uninstall_string
                    .as_deref()
                    .and_then(extract_product_code)
                    .or_else(|| entry.get_value("ProductCode").ok());
                let install_location: Option<String> = entry.get_value("InstallLocation").ok();
                let display_version: Option<String> = entry.get_value("DisplayVersion").ok();

                return Some(InstallRecord {
                    product_code,
                    uninstall_string,
                    install_location: install_location
                        .filter(|value| !value.trim().is_empty())
                        .map(PathBuf::from),
                    display_version,
                });
            }
        }
    }

    None
}

#[cfg(not(windows))]
fn find_install() -> Option<InstallRecord> {
    None
}

fn extract_product_code(uninstall_string: &str) -> Option<String> {
    let start = uninstall_string.find('{')?;
    let end = uninstall_string[start..].find('}')? + start;
    Some(uninstall_string[start..=end].to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn product_codes_are_found_in_msiexec_strings() {
        let value = "MsiExec.exe /I{12345678-1234-1234-1234-1234567890AB}";
        assert_eq!(
            extract_product_code(value).as_deref(),
            Some("{12345678-1234-1234-1234-1234567890AB}")
        );
    }

    #[test]
    fn checksum_validation_accepts_common_formats() {
        let hash = "A".repeat(64);
        assert_eq!(normalize_hash(&hash).unwrap(), hash.to_ascii_lowercase());
        assert!(normalize_hash(&format!("sha256:{hash}")).is_ok());
        assert!(normalize_hash("not-a-hash").is_err());
    }
}
