//! Elyra launcher: download, verify and install the latest Windows build.
//!
//! Running with no arguments downloads the newest MSI from GitHub Releases,
//! verifies it, installs it quietly and opens Elyra. The downloader is
//! resumable and atomic: a failed or interrupted transfer never replaces a
//! previously verified package. Explicit commands are available for scripted
//! or support use.

use reqwest::{blocking::Client, StatusCode, Url};
use sha2::{Digest, Sha256};
use std::{
    env, fmt::Write as _, fs,
    io::{self, Read, Write},
    path::{Path, PathBuf},
    process::{Command, ExitCode, Stdio},
    thread,
    time::{Duration, Instant},
};

const DEFAULT_DOWNLOAD_URL: &str =
    "https://github.com/imtiredwhenitalk/Elyra/releases/latest/download/Elyra-latest-x64.msi";
const MSI_MAGIC: [u8; 8] = [0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1];
const APP_NAME: &str = "Elyra";
const VERSION: &str = env!("CARGO_PKG_VERSION");
const KEYCHAIN_MARKER: &str = "com.elyra.desktop";

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CommandKind {
    Install,
    Download,
    Uninstall,
    Launch,
    Info,
    Cleanup,
    Help,
}

#[derive(Debug)]
struct Options {
    command: CommandKind,
    msi: Option<PathBuf>,
    url: String,
    sha256: Option<String>,
    product_code: Option<String>,
    backup_data: Option<PathBuf>,
    retries: u32,
    silent: bool,
    open_after_install: bool,
    download_only: bool,
    dry_run: bool,
    remove_data: bool,
    cleanup_downloads: bool,
    require_checksum: bool,
    allow_insecure_http: bool,
}

impl Options {
    fn parse(args: &[String]) -> Result<Self> {
        let mut options = Self {
            command: CommandKind::Install,
            msi: None,
            url: env::var("ELYRA_DOWNLOAD_URL").unwrap_or_else(|_| DEFAULT_DOWNLOAD_URL.to_string()),
            sha256: env::var("ELYRA_MSI_SHA256").ok(),
            product_code: None,
            backup_data: None,
            retries: 3,
            silent: false,
            open_after_install: true,
            download_only: false,
            dry_run: false,
            remove_data: false,
            cleanup_downloads: false,
            require_checksum: false,
            allow_insecure_http: false,
        };

        let mut index = 0;
        while index < args.len() {
            let argument = args[index].as_str();
            match argument {
                "install" | "update" => options.command = CommandKind::Install,
                "download" | "fetch" => options.command = CommandKind::Download,
                "uninstall" | "remove" => options.command = CommandKind::Uninstall,
                "launch" | "open" => options.command = CommandKind::Launch,
                "info" | "status" => options.command = CommandKind::Info,
                "cleanup" => options.command = CommandKind::Cleanup,
                "help" | "--help" | "-h" => options.command = CommandKind::Help,
                "--version" | "-V" => {
                    println!("Elyra Launcher {VERSION}");
                    std::process::exit(0);
                }
                "--silent" | "--quiet" | "-q" => options.silent = true,
                "--no-launch" => options.open_after_install = false,
                "--download-only" => options.download_only = true,
                "--dry-run" => options.dry_run = true,
                "--remove-data" | "--purge" => options.remove_data = true,
                "--cleanup" => options.cleanup_downloads = true,
                "--require-checksum" => options.require_checksum = true,
                "--allow-insecure-http" => options.allow_insecure_http = true,
                "--msi" | "--file" => {
                    index += 1;
                    let value = args.get(index).ok_or("--msi needs a path to an MSI file")?;
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
                "--product-code" => {
                    index += 1;
                    let value = args
                        .get(index)
                        .ok_or("--product-code needs a Windows Installer GUID")?;
                    options.product_code = Some(value.clone());
                }
                "--backup-data" => {
                    index += 1;
                    let value = args
                        .get(index)
                        .ok_or("--backup-data needs a destination folder")?;
                    options.backup_data = Some(PathBuf::from(value));
                }
                "--retries" => {
                    index += 1;
                    let value = args.get(index).ok_or("--retries needs a number")?;
                    options.retries = value
                        .parse::<u32>()
                        .map_err(|_| "--retries must be a number between 0 and 10")?;
                    if options.retries > 10 {
                        return Err("--retries must be a number between 0 and 10".into());
                    }
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
        if options.download_only {
            options.command = CommandKind::Download;
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
        CommandKind::Download => download_command(&options),
        CommandKind::Cleanup => cleanup_downloads(true),
        CommandKind::Uninstall => uninstall(&options),
        CommandKind::Launch => launch_app(),
        CommandKind::Install => install(&options),
    }
}

/// Download and verify a package. Returns the local path, whether it was
/// fetched in this run, and the expected SHA-256 when one is known.
fn acquire_package(options: &Options) -> Result<(PathBuf, bool, Option<String>)> {
    if let Some(path) = &options.msi {
        let path = path.canonicalize().unwrap_or_else(|_| path.clone());
        if !path.is_file() {
            return Err(format!("MSI not found: {}", path.display()).into());
        }
        println!("Using local package: {}", path.display());
        return Ok((path, false, options.sha256.clone()));
    }

    validate_url(&options.url, options.allow_insecure_http)?;
    let client = http_client()?;
    let directory = download_directory();
    fs::create_dir_all(&directory)?;
    let destination = directory.join("Elyra-latest-x64.msi");

    let expected = match &options.sha256 {
        Some(hash) => Some(hash.clone()),
        None => fetch_sidecar_hash(&client, &options.url),
    };

    if options.require_checksum && expected.is_none() {
        return Err("no SHA-256 checksum is published for this URL; pass --sha256 or remove --require-checksum".into());
    }

    if let Some(hash) = &expected {
        if destination.is_file() && sha256_file(&destination)? == *hash {
            println!("Reusing verified package: {}", destination.display());
            return Ok((destination, true, expected));
        }
    }

    println!("Downloading {}", options.url);
    download_with_retry(&client, &options.url, &destination, expected.as_deref(), options)?;
    Ok((destination, true, expected))
}

fn install(options: &Options) -> Result<()> {
    header("Install Elyra");

    let (msi_path, downloaded, expected) = acquire_package(options)?;
    validate_package(&msi_path)?;
    let actual_hash = sha256_file(&msi_path)?;

    match &expected {
        Some(expected) => {
            if actual_hash != *expected {
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
            if options.require_checksum {
                return Err("checksum verification was required but no digest was available".into());
            }
            println!("No published checksum was found; the MSI signature and Windows Installer still apply.");
        }
    }

    if options.dry_run {
        println!("\nDry run: package validated. Installation was skipped.");
        return Ok(());
    }

    install_package(&msi_path, options.silent)?;

    if options.cleanup_downloads && downloaded {
        let _ = fs::remove_file(&msi_path);
        println!("Removed the downloaded installer from {}", msi_path.display());
    }

    println!("\nElyra is installed.");
    if options.open_after_install {
        match launch_app() {
            Ok(()) => {}
            Err(error) => eprintln!("Could not open Elyra automatically: {error}"),
        }
    }
    Ok(())
}

fn download_command(options: &Options) -> Result<()> {
    header("Download Elyra installer");
    let (path, _, expected) = acquire_package(options)?;
    validate_package(&path)?;
    let hash = sha256_file(&path)?;

    if let Some(expected) = expected {
        if hash != expected {
            return Err(format!("checksum mismatch: expected {expected}, actual {hash}").into());
        }
    }

    println!("\nInstaller ready: {}", path.display());
    println!("SHA-256: {hash}");
    if options.dry_run {
        println!("Dry run: no changes were made.");
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

fn validate_url(url: &str, allow_insecure_http: bool) -> Result<Url> {
    let parsed = Url::parse(url)?;
    match parsed.scheme() {
        "https" => Ok(parsed),
        "http" if allow_insecure_http || is_loopback(parsed.host_str().unwrap_or_default()) => {
            Ok(parsed)
        }
        "http" => Err("plain HTTP downloads are disabled; use HTTPS or --allow-insecure-http".into()),
        scheme => Err(format!("unsupported URL scheme: {scheme}").into()),
    }
}

fn is_loopback(host: &str) -> bool {
    matches!(host, "localhost" | "127.0.0.1" | "[::1]" | "::1")
}

fn download_directory() -> PathBuf {
    env::temp_dir().join("elyra-installer")
}

fn cleanup_downloads(verbose: bool) -> Result<()> {
    let directory = download_directory();
    if !directory.is_dir() {
        if verbose {
            println!("No downloaded installers in {}.", directory.display());
        }
        return Ok(());
    }

    let mut removed = 0;
    for entry in fs::read_dir(&directory)? {
        let path = entry?.path();
        if !path.is_file() {
            continue;
        }
        let name = path.file_name().and_then(|value| value.to_str()).unwrap_or("");
        if !(name.starts_with("Elyra") || name.ends_with(".part") || name.ends_with(".msi")) {
            continue;
        }
        if fs::remove_file(&path).is_ok() {
            removed += 1;
            if verbose {
                println!("Removed {}", path.display());
            }
        }
    }
    if verbose {
        println!("Removed {removed} downloaded file(s).");
    }
    Ok(())
}

fn download_with_retry(
    client: &Client,
    url: &str,
    destination: &Path,
    expected: Option<&str>,
    options: &Options,
) -> Result<()> {
    let part = PathBuf::from(format!("{}.part", destination.display()));
    let attempts = options.retries + 1;
    let mut last_error: Option<String> = None;

    for attempt in 1..=attempts {
        match download_once(client, url, &part, expected, options) {
            Ok(()) => {
                let _ = fs::remove_file(destination);
                fs::rename(&part, destination)?;
                return Ok(());
            }
            Err(error) => {
                last_error = Some(error.to_string());
                if attempt == attempts {
                    break;
                }
                eprintln!("  Download attempt {attempt} failed: {error}");
                // A corrupt partial file cannot be resumed; a network error can.
                if let Some(hash) = expected {
                    if part.is_file()
                        && sha256_file(&part).map(|value| value != hash).unwrap_or(true)
                    {
                        let _ = fs::remove_file(&part);
                    }
                }
                thread::sleep(Duration::from_secs(attempt as u64));
            }
        }
    }

    let _ = fs::remove_file(&part);
    Err(format!(
        "download failed after {attempts} attempt(s): {}",
        last_error.unwrap_or_else(|| "unknown error".to_string())
    )
    .into())
}

fn download_once(
    client: &Client,
    url: &str,
    part: &Path,
    expected: Option<&str>,
    options: &Options,
) -> Result<()> {
    let mut offset = if part.is_file() { fs::metadata(part)?.len() } else { 0 };
    if offset > 0 {
        println!("  Resuming from {:.1} MB", offset as f64 / 1_048_576.0);
    }

    let mut request = client.get(url);
    if offset > 0 {
        request = request.header("Range", format!("bytes={offset}-"));
    }
    let mut response = request.send()?.error_for_status()?;
    let resume = offset > 0 && response.status() == StatusCode::PARTIAL_CONTENT;
    if !resume {
        offset = 0;
    }

    let total = response
        .headers()
        .get(reqwest::header::CONTENT_RANGE)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.rsplit('/').next())
        .and_then(|value| value.parse::<u64>().ok())
        .or_else(|| response.content_length().map(|length| length + offset));

    let mut hasher = Sha256::new();
    if resume {
        // Hash the already-downloaded prefix before appending new bytes.
        let mut prefix = fs::File::open(part)?;
        let mut buffer = [0_u8; 64 * 1024];
        loop {
            let read = prefix.read(&mut buffer)?;
            if read == 0 {
                break;
            }
            hasher.update(&buffer[..read]);
        }
        drop(prefix);
    }

    let mut file = if resume {
        fs::OpenOptions::new().append(true).open(part)?
    } else {
        fs::File::create(part)?
    };

    let mut buffer = [0_u8; 64 * 1024];
    let mut downloaded = offset;
    let mut last_report = offset;
    let started = Instant::now();
    loop {
        let read = response.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        file.write_all(&buffer[..read])?;
        hasher.update(&buffer[..read]);
        downloaded += read as u64;

        if downloaded - last_report >= 512 * 1024 {
            report_progress(downloaded, total, started.elapsed(), options.silent);
            last_report = downloaded;
        }
    }
    file.flush()?;
    drop(file);
    report_progress(downloaded, total, started.elapsed(), options.silent);
    if !options.silent {
        println!();
    }

    if downloaded == 0 || downloaded == offset {
        return Err("the download was empty".into());
    }

    let actual = to_hex(&hasher.finalize());
    if let Some(expected) = expected {
        if actual != expected {
            return Err(format!(
                "download checksum mismatch: expected {expected}, actual {actual}"
            )
            .into());
        }
    }

    Ok(())
}

fn report_progress(downloaded: u64, total: Option<u64>, elapsed: Duration, silent: bool) {
    if silent {
        return;
    }
    let downloaded_mb = downloaded as f64 / 1_048_576.0;
    let speed = if elapsed.as_secs_f64() > 0.0 {
        downloaded_mb / elapsed.as_secs_f64()
    } else {
        0.0
    };
    match total {
        Some(total) if total > 0 => {
            let percent = (downloaded as f64 / total as f64 * 100.0).min(100.0);
            print!("\r  {percent:5.1}%  {downloaded_mb:6.1} MB  {speed:5.1} MB/s");
        }
        _ => print!("\r  {downloaded_mb:6.1} MB  {speed:5.1} MB/s"),
    }
    let _ = io::stdout().flush();
}

fn fetch_sidecar_hash(client: &Client, url: &str) -> Option<String> {
    let checksum_url = format!("{url}.sha256");
    let response = client.get(checksum_url.as_str()).send().ok()?;
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

    let directory = download_directory();
    let _ = fs::create_dir_all(&directory);
    let log = directory.join("install.log");

    let status = Command::new("msiexec")
        .arg("/i")
        .arg(path)
        .arg(mode)
        .arg("/norestart")
        .arg("/log")
        .arg(&log)
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

    Err(format!(
        "Windows Installer exited with {status}; see {}",
        log.display()
    )
    .into())
}

fn data_directory() -> Option<PathBuf> {
    env::var_os("LOCALAPPDATA").map(|value| PathBuf::from(value).join("com.elyra.desktop"))
}

fn uninstall(options: &Options) -> Result<()> {
    header("Remove Elyra");

    let record = find_install(options.product_code.as_deref());
    let Some(record) = record else {
        println!("Elyra is not registered for this user. Nothing to remove.");
        return cleanup_local_data(options);
    };

    if let Some(version) = &record.display_version {
        println!("Found Elyra {version}");
    }

    if options.dry_run {
        println!("Dry run: no changes were made.");
        println!("Would remove the registered Elyra installation.");
        report_local_data(options);
        return Ok(());
    }

    if let Some(product_code) = &record.product_code {
        let mode = if options.silent { "/quiet" } else { "/passive" };
        let status = Command::new("msiexec")
            .args(["/x", product_code.as_str(), mode, "/norestart"])
            .stdin(Stdio::null())
            .status()?;
        if !(status.success() || status.code() == Some(3010)) {
            return Err(format!("Windows Installer exited with {status}").into());
        }
    } else if let Some(uninstall_string) = &record.uninstall_string {
        let status = Command::new("cmd")
            .args(["/C", uninstall_string.as_str()])
            .stdin(Stdio::null())
            .status()?;
        if !status.success() {
            return Err(format!("The uninstaller exited with {status}").into());
        }
    } else {
        return Err("Elyra's uninstall entry has no product code or uninstall command".into());
    }

    println!("Elyra was removed.");
    cleanup_local_data(options)
}

fn report_local_data(options: &Options) {
    if let Some(path) = data_directory() {
        println!("Local data : {} (kept by default)", path.display());
    }
    if options.remove_data {
        println!("--remove-data would delete local data and Elyra keychain entries.");
    } else {
        println!("Run with --remove-data to delete local data and keychain entries.");
    }
}

fn cleanup_local_data(options: &Options) -> Result<()> {
    if !options.remove_data {
        println!("Your local mailbox database was left in place.");
        return Ok(());
    }

    if options.dry_run {
        report_local_data(options);
        return Ok(());
    }

    if let Some(destination) = &options.backup_data {
        backup_data(destination)?;
    }

    let data = data_directory();
    if let Some(path) = data {
        if path.is_dir() {
            fs::remove_dir_all(&path)?;
            println!("Removed local data: {}", path.display());
        } else {
            println!("No local data folder was found.");
        }
    } else {
        println!("LOCALAPPDATA is not set; no local data folder was found.");
    }

    let removed = remove_keychain_entries()?;
    println!("Removed {removed} Elyra credential(s) from the OS keychain.");
    cleanup_downloads(false)?;
    Ok(())
}

fn backup_data(destination: &Path) -> Result<()> {
    let Some(source) = data_directory() else {
        return Err("LOCALAPPDATA is not set; cannot locate Elyra data".into());
    };
    if !source.is_dir() {
        println!("Nothing to back up: {} does not exist.", source.display());
        return Ok(());
    }

    fs::create_dir_all(destination)?;
    let mut copied = 0;
    for entry in fs::read_dir(&source)? {
        let path = entry?.path();
        if !path.is_file() {
            continue;
        }
        let name = path.file_name().ok_or("a data file has no file name")?;
        fs::copy(&path, destination.join(name))?;
        copied += 1;
    }
    println!(
        "Backed up {copied} file(s) from {} to {}",
        source.display(),
        destination.display()
    );
    Ok(())
}

#[cfg(windows)]
fn remove_keychain_entries() -> Result<usize> {
    use std::process::Output;

    let output: Output = Command::new("cmdkey").arg("/list").output()?;
    if !output.status.success() {
        eprintln!("Could not list Windows Credential Manager entries.");
        return Ok(0);
    }

    let text = String::from_utf8_lossy(&output.stdout);
    let targets: Vec<String> = text
        .lines()
        .filter_map(|line| line.trim().strip_prefix("Target:"))
        .map(str::trim)
        .filter(|target| is_elyra_target(target))
        .map(str::to_string)
        .collect();

    let mut removed = 0;
    for target in targets {
        let status = Command::new("cmdkey")
            .arg(format!("/delete:{target}"))
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .status()?;
        if status.success() {
            removed += 1;
        } else {
            eprintln!("Could not remove credential target: {target}");
        }
    }
    Ok(removed)
}

#[cfg(not(windows))]
fn remove_keychain_entries() -> Result<usize> {
    Ok(0)
}

/// keyring-rs maps `Entry::new("com.elyra.desktop", user)` to a credential
/// target containing both parts, in either order depending on the backend.
fn is_elyra_target(target: &str) -> bool {
    target.to_ascii_lowercase().contains(KEYCHAIN_MARKER)
}

fn print_help() {
    println!(
        "Elyra Launcher {VERSION}

USAGE
  elyra-launcher                 Download, verify, install and open Elyra
  elyra-launcher install         Same as running with no arguments
  elyra-launcher download        Download and verify the installer only
  elyra-launcher uninstall       Remove Elyra, keeping the local mailbox
  elyra-launcher launch          Open an installed Elyra
  elyra-launcher info            Show launcher and install details
  elyra-launcher cleanup         Delete stale downloaded installers

OPTIONS
  --msi <path>                   Install a local MSI instead of downloading
  --url <url>                    Use a custom MSI URL (HTTPS by default)
  --sha256 <digest>              Require a specific SHA-256 checksum
  --require-checksum             Refuse to install without a published checksum
  --retries <n>                  Download attempts, 0-10 (default: 3)
  --download-only                Alias for the download command
  --dry-run                      Validate and report without changing anything
  --silent                       Fully quiet installer (default is /passive)
  --no-launch                    Do not open Elyra after installing
  --cleanup                      Remove the downloaded MSI after installing
  --remove-data                  Uninstall: also delete local data and credentials
  --backup-data <folder>         Copy local database files before --remove-data
  --product-code <GUID>          Uninstall a specific Windows Installer product
  --allow-insecure-http          Permit plain HTTP downloads (not recommended)
  --version                      Print the launcher version
  -h, --help                     Show this help

ENVIRONMENT
  ELYRA_DOWNLOAD_URL             Default MSI URL
  ELYRA_MSI_SHA256               Expected SHA-256 checksum

Downloads use a .part file, resume interrupted transfers, retry temporary
failures, and never replace a verified installer with a partial download. A
sibling <url>.sha256 file is used automatically when it is published."
    );
}

fn print_info() {
    header("Elyra Launcher");
    println!("Launcher version : {VERSION}");
    println!("Platform         : {} {}", env::consts::OS, env::consts::ARCH);
    println!("Download URL    : {}", DEFAULT_DOWNLOAD_URL);
    println!("Download folder : {}", download_directory().display());
    println!(
        "Local data      : {}",
        data_directory()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| "not available".to_string())
    );

    match find_app_executable() {
        Some(path) => println!("Application     : {}", path.display()),
        None => println!("Application     : not found"),
    }

    match find_install(None) {
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

    if let Some(record) = find_install(None) {
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
fn find_install(product_code: Option<&str>) -> Option<InstallRecord> {
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
                let matches_name = display_name.eq_ignore_ascii_case(APP_NAME)
                    || display_name.starts_with(&format!("{APP_NAME} "));
                if product_code.is_none() && !matches_name {
                    continue;
                }

                let uninstall_string: Option<String> = entry.get_value("UninstallString").ok();
                let found_code = uninstall_string
                    .as_deref()
                    .and_then(extract_product_code)
                    .or_else(|| entry.get_value("ProductCode").ok());
                if let Some(requested) = product_code {
                    if found_code.as_deref() != Some(requested) {
                        continue;
                    }
                }

                let install_location: Option<String> = entry.get_value("InstallLocation").ok();
                let display_version: Option<String> = entry.get_value("DisplayVersion").ok();

                return Some(InstallRecord {
                    product_code: found_code,
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
fn find_install(_product_code: Option<&str>) -> Option<InstallRecord> {
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

    #[test]
    fn https_is_default_and_loopback_http_is_opt_in() {
        let https = "https://example.com/Elyra.msi";
        assert!(validate_url(https, false).is_ok());
        assert!(validate_url("http://example.com/Elyra.msi", false).is_err());
        assert!(validate_url("http://example.com/Elyra.msi", true).is_ok());
        assert!(validate_url("http://127.0.0.1:8080/Elyra.msi", false).is_ok());
        assert!(validate_url("ftp://example.com/Elyra.msi", true).is_err());
    }

    #[test]
    fn keychain_matching_accepts_both_keyring_target_orders() {
        assert!(is_elyra_target("cloud_session.com.elyra.desktop"));
        assert!(is_elyra_target("com.elyra.desktop:integration-virustotal"));
        assert!(!is_elyra_target("github.com:another-app"));
    }

    #[test]
    fn download_command_flag_selects_download_only() {
        let options = Options::parse(&["--download-only".to_string()]).unwrap();
        assert_eq!(options.command, CommandKind::Download);
    }
}
