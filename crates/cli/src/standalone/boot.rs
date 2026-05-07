use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use uzumaki_runtime::AppConfig;

use super::embed::read_payload_from_current_exe;
use super::format::{StandalonePayload, deserialize_payload_bytes, read_payload_from_exe};
use super::vfs::extract;

const DEFAULT_IDENTIFIER: &str = "com.uzumaki.app";

#[derive(Debug, Clone)]
pub enum LaunchMode {
    Dev {
        config: AppConfig,
    },
    Standalone {
        config: AppConfig,
        #[allow(dead_code)]
        extraction_dir: PathBuf,
    },
}

impl LaunchMode {
    pub fn app_config(&self) -> &AppConfig {
        match self {
            LaunchMode::Dev { config, .. } => config,
            LaunchMode::Standalone { config, .. } => config,
        }
    }
}

/// Detect whether the current executable contains an embedded standalone
/// payload. If yes, extract it (idempotently) and return a Standalone launch
/// mode. Otherwise return `Ok(None)` so the caller can fall back to dev mode.
pub fn detect_and_prepare() -> Result<Option<LaunchMode>> {
    let exe = std::env::current_exe().context("resolving current_exe")?;
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    let Some(payload) = load_payload(&exe)? else {
        return Ok(None);
    };

    let extraction_dir = choose_extraction_dir(&exe, &payload)?;
    ensure_extracted(&payload, &extraction_dir)?;

    let app_root = extraction_dir.join(&payload.metadata.dist_root_dir_name);
    // `entry_rel_path` already includes the dist_root_dir_name as prefix
    // (e.g. "app/index.js"); resolve against extraction_dir for a stable path.
    let entry_path = extraction_dir.join(
        payload
            .metadata
            .entry_rel_path
            .replace('/', std::path::MAIN_SEPARATOR_STR),
    );

    let exe_dir = exe.parent().map(Path::to_path_buf).unwrap_or_default();
    // macOS: exe lives at `<App>.app/Contents/MacOS/<binary>`, resources at
    // `<App>.app/Contents/Resources/`. Elsewhere: `<exe_dir>/resources/`.
    let resource_root = if cfg!(target_os = "macos")
        && exe_dir.file_name().and_then(|n| n.to_str()) == Some("MacOS")
    {
        exe_dir
            .parent()
            .map(|p| p.join("Resources"))
            .unwrap_or_else(|| exe_dir.join("resources"))
    } else {
        exe_dir.join("resources")
    };

    Ok(Some(LaunchMode::Standalone {
        config: AppConfig {
            entry: entry_path,
            app_root,
            args,
            identifier: DEFAULT_IDENTIFIER.to_string(),
            resource_root,
        },
        extraction_dir,
    }))
}

/// Resolve the embedded payload for the current executable.
///
/// Tries the production path first — a real PE resource / Mach-O section /
/// ELF note section read via libsui — and falls back to the legacy v1
/// trailer-append format so existing packed binaries keep booting during
/// the transition.
fn load_payload(exe: &Path) -> Result<Option<StandalonePayload>> {
    match read_payload_from_current_exe() {
        Ok(Some(bytes)) => match deserialize_payload_bytes(&bytes) {
            Ok(payload) => return Ok(Some(payload)),
            Err(e) => {
                eprintln!(
                    "uzumaki: native section present but failed to deserialize: {e}; \
                     falling back to legacy trailer reader"
                );
            }
        },
        Ok(None) => {}
        Err(e) => {
            eprintln!(
                "uzumaki: native section lookup failed: {e}; \
                 falling back to legacy trailer reader"
            );
        }
    }

    read_payload_from_exe(exe)
}

fn choose_extraction_dir(exe: &Path, payload: &StandalonePayload) -> Result<PathBuf> {
    let hash = &payload.metadata.extract_hash;
    let exe_stem = exe
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("uzumaki_app");

    let base = dirs::cache_dir().unwrap_or_else(std::env::temp_dir);
    Ok(base.join("uzumaki").join(exe_stem).join(hash))
}

fn ensure_extracted(payload: &StandalonePayload, extraction_dir: &Path) -> Result<()> {
    let done_marker = extraction_dir.join(".done");
    if done_marker.exists() {
        return Ok(());
    }

    fs::create_dir_all(extraction_dir)
        .with_context(|| format!("creating {}", extraction_dir.display()))?;
    extract(payload, extraction_dir)?;
    fs::write(&done_marker, b"1")
        .with_context(|| format!("writing done marker {}", done_marker.display()))?;
    Ok(())
}
