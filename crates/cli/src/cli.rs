use anyhow::{Context, Result, bail};
use clap::{CommandFactory, Parser, Subcommand};
use std::fs;
use std::io::Read as _;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::standalone;
use uzumaki_runtime::AppConfig;

const VERSION: &str = env!("CARGO_PKG_VERSION");
const GITHUB_REPO: &str = "golok727/uzumaki";

#[derive(Debug, serde::Deserialize)]
pub struct UzumakiConfig {
    #[serde(rename = "productName")]
    pub product_name: String,
    pub version: String,
    pub identifier: String,
    #[serde(default)]
    pub build: BuildConfig,
    #[serde(default)]
    pub pack: PackConfig,
    #[serde(default)]
    pub bundle: BundleConfig,
}

#[derive(Debug, Default, serde::Deserialize)]
pub struct BundleConfig {
    /// Files / globs to copy next to the packed exe under `resources/`.
    /// Resolved relative to the config file's directory.
    #[serde(default)]
    pub resources: Vec<String>,
}

#[derive(Debug, Default, serde::Deserialize)]
pub struct BuildConfig {
    pub command: Option<String>,
}

#[derive(Debug, Default, serde::Deserialize)]
pub struct PackConfig {
    pub dist: Option<String>,
    pub entry: Option<String>,
    pub output: Option<String>,
    pub name: Option<String>,
    #[serde(rename = "baseBinary")]
    pub base_binary: Option<String>,
}

fn find_config(start: &Path) -> Option<PathBuf> {
    let mut dir = start.to_path_buf();
    loop {
        let candidate = dir.join("uzumaki.config.json");
        if candidate.is_file() {
            return Some(candidate);
        }
        if !dir.pop() {
            return None;
        }
    }
}

fn load_config(path: &Path) -> Result<UzumakiConfig> {
    let raw = fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    serde_json::from_str(&raw).with_context(|| format!("parsing {}", path.display()))
}

#[derive(Parser)]
#[command(
    name = "uzumaki",
    about = "\x1b[1;38;5;75mUzumaki\x1b[0m — Desktop UI Framework",
    version = VERSION,
    styles = clap_styles(),
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Run a JS/TS file in the uzumaki runtime
    Run {
        /// Entry point file
        entry: String,
        /// Extra arguments passed to the runtime
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Build and package an app using uzumaki.config.json
    Build {
        /// Path to config file
        #[arg(long)]
        config: Option<String>,
        /// Skip the build step
        #[arg(long)]
        no_build: bool,
    },
    /// Initialize a new Uzumaki project
    Init {
        /// Directory to create the project in (defaults to project name)
        directory: Option<String>,
    },
    /// Upgrade uzumaki to the latest version
    Upgrade {
        /// Specific version to install (e.g. 0.2.0)
        #[arg(long)]
        version: Option<String>,
    },
}

fn clap_styles() -> clap::builder::Styles {
    clap::builder::Styles::styled()
        .header(
            clap::builder::styling::AnsiColor::BrightCyan
                .on_default()
                .bold(),
        )
        .usage(
            clap::builder::styling::AnsiColor::BrightCyan
                .on_default()
                .bold(),
        )
        .literal(clap::builder::styling::AnsiColor::BrightBlue.on_default())
        .placeholder(clap::builder::styling::AnsiColor::White.on_default())
}

/// Known subcommand names so we can distinguish `uzumaki build` from `uzumaki app.tsx`.
const KNOWN_SUBCOMMANDS: &[&str] = &["run", "build", "init", "upgrade", "help"];

pub fn run_cli() -> Result<Option<standalone::LaunchMode>> {
    let raw_args: Vec<String> = std::env::args().collect();

    if raw_args.len() <= 1 {
        Cli::command().print_help().ok();
        println!();
        return Ok(None);
    }

    // If the first arg after the binary name looks like a file (not a known subcommand),
    // treat it as `uzumaki run <file> ...`
    let cli = if !KNOWN_SUBCOMMANDS.contains(&raw_args[1].as_str()) && !raw_args[1].starts_with('-')
    {
        let mut patched = vec![raw_args[0].clone(), "run".to_string()];
        patched.extend_from_slice(&raw_args[1..]);
        Cli::parse_from(patched)
    } else {
        Cli::parse()
    };

    match cli.command {
        Commands::Run { entry, args } => Ok(Some(resolve_run(&entry, args)?)),
        Commands::Build { config, no_build } => {
            cmd_build(config.as_deref(), no_build)?;
            Ok(None)
        }
        Commands::Init { directory } => {
            crate::init::cmd_init(directory.as_deref())?;
            Ok(None)
        }
        Commands::Upgrade { version } => {
            cmd_upgrade(version.as_deref())?;
            Ok(None)
        }
    }
}

fn resolve_run(entry: &str, args: Vec<String>) -> Result<standalone::LaunchMode> {
    let cwd = std::env::current_dir()?;
    let entry_path = strip_unc_prefix(
        fs::canonicalize(cwd.join(entry))
            .with_context(|| format!("entry point not found: {entry}"))?,
    );
    let app_root = entry_path
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or(cwd.clone());

    // Locate the config to derive identifier and the resource root for dev
    // mode. In dev, resources are read straight from the project tree, so the
    // resource root is the config file's directory (or app_root as a fallback).
    let (identifier, resource_root) = match find_config(&app_root) {
        Some(config_path) => {
            let config_dir = config_path
                .parent()
                .map(|p| p.to_path_buf())
                .unwrap_or_else(|| app_root.clone());
            let identifier = load_config(&config_path)
                .map(|c| c.identifier)
                .unwrap_or_else(|_| "com.uzumaki.app".to_string());
            (identifier, config_dir)
        }
        None => ("com.uzumaki.app".to_string(), app_root.clone()),
    };

    Ok(standalone::LaunchMode::Dev {
        config: AppConfig {
            entry: entry_path,
            app_root,
            args,
            identifier,
            resource_root,
        },
    })
}

/// On Windows, `fs::canonicalize` returns a `\\?\C:\...` extended-length path.
/// Strip the prefix when it's safe (regular drive paths) so user-visible strings
/// and `Uz.path.resource(...)` outputs look like normal `C:\...` paths.
fn strip_unc_prefix(path: PathBuf) -> PathBuf {
    if cfg!(windows) {
        let s = path.to_string_lossy();
        if let Some(rest) = s.strip_prefix(r"\\?\")
            && !rest.starts_with(r"UNC\")
        {
            return PathBuf::from(rest);
        }
    }
    path
}

fn cmd_build(config_path: Option<&str>, no_build: bool) -> Result<()> {
    let cwd = std::env::current_dir()?;

    let config_file = match config_path {
        Some(p) => {
            let p = cwd.join(p);
            if !p.is_file() {
                bail!("config file not found: {}", p.display());
            }
            p
        }
        None => find_config(&cwd).ok_or_else(|| {
            anyhow::anyhow!("could not find uzumaki.config.json from {}", cwd.display())
        })?,
    };

    let config_dir = config_file.parent().unwrap().to_path_buf();
    let config = load_config(&config_file)?;

    if !no_build && let Some(ref cmd) = config.build.command {
        println!(
            "\x1b[1;38;5;75muzumaki\x1b[0m \x1b[2mrunning build:\x1b[0m {}",
            cmd
        );
        let status = run_shell_command(cmd, &config_dir)?;
        if !status.success() {
            bail!("build command failed with exit code {}", status);
        }
    }

    // Pack
    let dist = config
        .pack
        .dist
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("missing pack.dist in config"))?;
    let entry = config
        .pack
        .entry
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("missing pack.entry in config"))?;
    let output_raw = config
        .pack
        .output
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("missing pack.output in config"))?;

    let dist_path = resolve_from(&config_dir, dist);
    let output_path = normalize_output_extension(&resolve_from(&config_dir, output_raw));
    let app_name = config
        .pack
        .name
        .clone()
        .unwrap_or_else(|| config.product_name.clone());
    let base_binary = match &config.pack.base_binary {
        Some(b) => resolve_from(&config_dir, b),
        None => std::env::current_exe()?,
    };

    println!(
        "\x1b[1;38;5;75muzumaki\x1b[0m \x1b[2mpacking\x1b[0m {} → {}",
        dist,
        output_path.display()
    );

    let final_output = standalone::pack::pack_app(&standalone::pack::PackOptions {
        dist_dir: dist_path,
        entry_rel: entry.to_string(),
        output: output_path.clone(),
        app_name,
        base_binary,
        identifier: config.identifier,
        version: config.version,
        product_name: config.product_name,
    })?;

    if !config.bundle.resources.is_empty() {
        let resources_dir = resources_dir_for(&final_output)?;
        copy_bundle_resources(&config_dir, &config.bundle.resources, &resources_dir)?;
    }

    Ok(())
}

/// Where bundle resources land relative to the packed artifact.
/// - macOS: `<App>.app/Contents/Resources/` (already created by the bundler).
/// - Windows / Linux: `<exe_dir>/resources/` (created on demand).
fn resources_dir_for(final_output: &Path) -> Result<PathBuf> {
    if cfg!(target_os = "macos") {
        Ok(final_output.join("Contents").join("Resources"))
    } else {
        final_output
            .parent()
            .map(|p| p.join("resources"))
            .ok_or_else(|| anyhow::anyhow!("output path has no parent"))
    }
}

fn copy_bundle_resources(base: &Path, patterns: &[String], resources_dir: &Path) -> Result<()> {
    fs::create_dir_all(resources_dir)
        .with_context(|| format!("creating {}", resources_dir.display()))?;

    for pattern in patterns {
        let mut matched = false;
        let abs_pattern = if Path::new(pattern).is_absolute() {
            pattern.clone()
        } else {
            base.join(pattern).to_string_lossy().into_owned()
        };

        for entry in
            glob::glob(&abs_pattern).with_context(|| format!("invalid glob pattern: {pattern}"))?
        {
            let path = entry.with_context(|| format!("globbing {pattern}"))?;
            matched = true;
            let rel = path.strip_prefix(base).unwrap_or(&path);
            let dest = resources_dir.join(rel);
            copy_path(&path, &dest)?;
        }

        if !matched {
            eprintln!(
                "\x1b[1;33muzumaki\x1b[0m \x1b[2mbundle resource\x1b[0m {} \x1b[33mmatched nothing\x1b[0m",
                pattern
            );
        }
    }
    Ok(())
}

fn copy_path(src: &Path, dest: &Path) -> Result<()> {
    let meta = fs::metadata(src).with_context(|| format!("stat {}", src.display()))?;
    if meta.is_dir() {
        fs::create_dir_all(dest).with_context(|| format!("creating {}", dest.display()))?;
        for entry in fs::read_dir(src).with_context(|| format!("reading {}", src.display()))? {
            let entry = entry?;
            let name = entry.file_name();
            copy_path(&entry.path(), &dest.join(&name))?;
        }
    } else {
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;
        }
        fs::copy(src, dest)
            .with_context(|| format!("copying {} -> {}", src.display(), dest.display()))?;
    }
    Ok(())
}

fn cmd_upgrade(target_version: Option<&str>) -> Result<()> {
    println!("\x1b[1;38;5;75muzumaki\x1b[0m \x1b[2mchecking for updates...\x1b[0m");

    let version_tag = match target_version {
        Some(v) => {
            if v.starts_with('v') {
                v.to_string()
            } else {
                format!("v{v}")
            }
        }
        None => {
            // Fetch latest release tag from GitHub API
            let url = format!("https://api.github.com/repos/{GITHUB_REPO}/releases/latest");
            let body: String = ureq::get(&url)
                .header("Accept", "application/vnd.github+json")
                .header("User-Agent", "uzumaki-updater")
                .call()
                .context("failed to fetch latest release")?
                .body_mut()
                .read_to_string()
                .context("failed to read response body")?;
            let release: serde_json::Value =
                serde_json::from_str(&body).context("invalid JSON from GitHub API")?;
            release["tag_name"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("no tag_name in latest release"))?
                .to_string()
        }
    };

    let version_num = version_tag.strip_prefix('v').unwrap_or(&version_tag);

    if version_num == VERSION {
        println!("\x1b[1;38;5;75muzumaki\x1b[0m \x1b[32malready up to date\x1b[0m (v{VERSION})");
        return Ok(());
    }

    let asset_name = get_asset_name();
    let download_url =
        format!("https://github.com/{GITHUB_REPO}/releases/download/{version_tag}/{asset_name}");

    println!("\x1b[1;38;5;75muzumaki\x1b[0m \x1b[2mdownloading\x1b[0m v{VERSION} → v{version_num}");

    let mut response = ureq::get(&download_url)
        .header("User-Agent", "uzumaki-updater")
        .call()
        .with_context(|| format!("failed to download {download_url}"))?;

    let total = response
        .headers()
        .get("content-length")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(0);

    let mut body_bytes = Vec::with_capacity(total as usize);
    let mut reader = response.body_mut().as_reader();
    let mut downloaded: u64 = 0;
    let mut buf = [0u8; 64 * 1024];
    loop {
        let n = reader
            .read(&mut buf)
            .context("failed to read download body")?;
        if n == 0 {
            break;
        }
        body_bytes.extend_from_slice(&buf[..n]);
        downloaded += n as u64;
        if total > 0 {
            let pct = (downloaded as f64 / total as f64 * 100.0) as u8;
            let filled = (pct as usize) / 2;
            eprint!(
                "\r\x1b[1;38;5;75muzumaki\x1b[0m \x1b[2m[{:█<filled$}{:·<empty$}] {pct}%\x1b[0m",
                "",
                "",
                filled = filled,
                empty = 50 - filled,
                pct = pct,
            );
        }
    }
    if total > 0 {
        eprintln!();
    }

    let binary_bytes = extract_binary_from_zip(&body_bytes, &get_binary_name())?;
    let current_exe = std::env::current_exe()?;
    replace_exe(&current_exe, &binary_bytes)?;

    println!("\x1b[1;38;5;75muzumaki\x1b[0m \x1b[32mupdated to v{version_num}\x1b[0m");

    Ok(())
}

fn extract_binary_from_zip(zip_bytes: &[u8], binary_name: &str) -> Result<Vec<u8>> {
    let reader = std::io::Cursor::new(zip_bytes);
    let mut archive = zip::ZipArchive::new(reader).context("invalid zip archive")?;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let name = file.name().to_string();
        if name == binary_name || name.ends_with(&format!("/{binary_name}")) {
            let mut bytes = Vec::with_capacity(file.size() as usize);
            std::io::Read::read_to_end(&mut file, &mut bytes)?;
            return Ok(bytes);
        }
    }

    bail!("binary '{binary_name}' not found in zip archive")
}

fn replace_exe(current_exe: &Path, new_bytes: &[u8]) -> Result<()> {
    let dir = current_exe.parent().unwrap();
    let tmp_file = tempfile::NamedTempFile::new_in(dir)?;
    fs::write(tmp_file.path(), new_bytes)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(tmp_file.path(), fs::Permissions::from_mode(0o755))?;
    }

    let backup_path = current_exe.with_extension("old");
    let _ = fs::remove_file(&backup_path);

    fs::rename(current_exe, &backup_path)
        .with_context(|| format!("failed to move current exe to {}", backup_path.display()))?;

    if let Err(e) = fs::rename(tmp_file.path(), current_exe) {
        // Rollback
        let _ = fs::rename(&backup_path, current_exe);
        return Err(e).context("failed to place new binary");
    }

    tmp_file.into_temp_path().keep()?;

    // Clean up backup
    let _ = fs::remove_file(&backup_path);

    Ok(())
}

fn resolve_from(base: &Path, value: &str) -> PathBuf {
    let p = Path::new(value);
    let joined = if p.is_absolute() {
        p.to_path_buf()
    } else {
        base.join(p)
    };
    normalize_path(&joined)
}

fn normalize_path(path: &Path) -> PathBuf {
    use std::path::Component;
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            other => out.push(other),
        }
    }
    out
}

fn normalize_output_extension(path: &Path) -> PathBuf {
    let s = path.to_string_lossy();
    if cfg!(target_os = "windows") {
        if s.ends_with(".exe") {
            path.to_path_buf()
        } else {
            PathBuf::from(format!("{s}.exe"))
        }
    } else if cfg!(target_os = "macos") {
        let cleaned = s.trim_end_matches(".exe").trim_end_matches(".app");
        PathBuf::from(cleaned.to_string())
    } else {
        // Linux – strip .exe if present
        let cleaned = s.trim_end_matches(".exe");
        PathBuf::from(cleaned.to_string())
    }
}

fn run_shell_command(command: &str, cwd: &Path) -> Result<std::process::ExitStatus> {
    let status = if cfg!(target_os = "windows") {
        Command::new("cmd.exe")
            .args(["/d", "/s", "/c", command])
            .current_dir(cwd)
            .status()
    } else {
        Command::new("sh")
            .args(["-lc", command])
            .current_dir(cwd)
            .status()
    };
    status.with_context(|| format!("failed to run: {command}"))
}

fn get_binary_name() -> String {
    if cfg!(target_os = "windows") {
        "uzumaki.exe".to_string()
    } else {
        "uzumaki".to_string()
    }
}

fn get_asset_name() -> String {
    let os = if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "macos") {
        "macos"
    } else {
        "linux"
    };

    let arch = if cfg!(target_arch = "x86_64") {
        "x64"
    } else if cfg!(target_arch = "aarch64") {
        "arm64"
    } else {
        "x64"
    };

    format!("uzumaki-{os}-{arch}.zip")
}
