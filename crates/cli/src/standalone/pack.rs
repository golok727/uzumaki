use anyhow::{Context, Result, anyhow, bail};
use std::fs;
use std::path::{Path, PathBuf};

use super::embed::write_payload_into_exe;
use super::format::{
    FORMAT_VERSION, StandaloneMetadata, VfsEntry, fnv1a_hex, read_payload_from_exe,
    serialize_payload_bytes,
};
use super::vfs::walk_dir;

pub struct PackOptions {
    pub dist_dir: PathBuf,
    pub entry_rel: String,
    pub output: PathBuf,
    pub app_name: String,
    pub base_binary: PathBuf,
    pub identifier: String,
    pub version: String,
    pub product_name: String,
}

pub fn pack_app(opts: &PackOptions) -> Result<PathBuf> {
    if !opts.dist_dir.is_dir() {
        bail!("dist directory does not exist: {}", opts.dist_dir.display());
    }
    // Normalize entry to forward slashes for lookup within manifest.
    let entry_rel_norm = opts.entry_rel.replace('\\', "/");

    let files = walk_dir(&opts.dist_dir)?;
    if files.is_empty() {
        bail!("dist directory is empty: {}", opts.dist_dir.display());
    }

    if !files.iter().any(|(rel, _)| rel == &entry_rel_norm) {
        bail!(
            "entry `{}` was not found inside {}",
            entry_rel_norm,
            opts.dist_dir.display()
        );
    }

    // Build manifest + blob
    let mut blob: Vec<u8> = Vec::new();
    let mut manifest: Vec<VfsEntry> = Vec::with_capacity(files.len());
    for (rel, abs) in &files {
        let bytes = fs::read(abs).with_context(|| format!("reading {}", abs.display()))?;
        let offset = blob.len() as u64;
        let len = bytes.len() as u64;
        blob.extend_from_slice(&bytes);
        manifest.push(VfsEntry {
            relative_path: rel.clone(),
            offset,
            len,
            executable: false,
        });
    }

    // Read the base runtime binary. If it itself happens to be a previously
    // trailer-packed exe (v1 format), strip the trailer so we don't carry
    // stale junk into the new container.
    let mut base_bytes = read_base_exe_without_payload(&opts.base_binary)?;

    // If this is a Windows PE, flip the subsystem to GUI so double-clicking
    // the packed executable doesn't pop a console window. This mirrors Deno's
    // `set_windows_binary_to_gui` in cli/standalone/binary.rs and must happen
    // *before* we hand the bytes to libsui for resource embedding.
    if is_pe(&base_bytes) {
        set_windows_binary_to_gui(&mut base_bytes)?;
    }

    // Compute extract hash from manifest + blob (stable, deterministic).
    let manifest_json_for_hash = serde_json::to_vec(&manifest)?;
    let mut hash_input = Vec::with_capacity(manifest_json_for_hash.len() + blob.len());
    hash_input.extend_from_slice(&manifest_json_for_hash);
    hash_input.extend_from_slice(&blob);
    let extract_hash = fnv1a_hex(&hash_input);

    let metadata = StandaloneMetadata {
        format_version: FORMAT_VERSION,
        app_name: opts.app_name.clone(),
        entry_rel_path: format!("app/{}", entry_rel_norm),
        dist_root_dir_name: "app".to_string(),
        extract_hash,
    };

    // Build the self-contained payload bytes and embed them in a real
    // PE resource / Mach-O section / ELF note section.
    let payload = serialize_payload_bytes(&metadata, &manifest, &blob)?;

    let final_output = if cfg!(target_os = "macos") {
        create_macos_app_bundle(
            base_bytes,
            payload,
            &opts.output,
            &opts.app_name,
            &opts.identifier,
            &opts.version,
            &opts.product_name,
        )?
    } else {
        write_payload_into_exe(base_bytes, payload, &opts.output)?;
        opts.output.clone()
    };

    println!(
        "packed {} file(s) into {}",
        files.len(),
        final_output.display()
    );
    Ok(final_output)
}

/// Returns `true` if `bytes` looks like a Windows PE executable (has a valid
/// MZ header and a PE\0\0 signature at the offset stored in `e_lfanew`).
fn is_pe(bytes: &[u8]) -> bool {
    if bytes.len() < 0x40 || &bytes[0..2] != b"MZ" {
        return false;
    }
    let pe_off = u32::from_le_bytes(match bytes[0x3C..0x40].try_into() {
        Ok(b) => b,
        Err(_) => return false,
    }) as usize;
    bytes
        .get(pe_off..pe_off + 4)
        .map(|s| s == b"PE\0\0")
        .unwrap_or(false)
}

/// Patch a PE's Optional Header Subsystem field from CUI (3) to GUI (2) so
/// double-clicking the packed executable on Windows does not spawn a console
/// window. Port of Deno's `set_windows_binary_to_gui`
/// (`cli/standalone/binary.rs`). See the PE format reference:
/// https://learn.microsoft.com/en-us/windows/win32/debug/pe-format
fn set_windows_binary_to_gui(bin: &mut [u8]) -> Result<()> {
    // e_lfanew sits at offset 0x3C and points at the PE signature.
    let pe_off = u32::from_le_bytes(
        bin[0x3C..0x40]
            .try_into()
            .map_err(|_| anyhow!("PE header offset slice error"))?,
    ) as usize;

    if bin
        .get(pe_off..pe_off + 4)
        .map(|s| s != b"PE\0\0")
        .unwrap_or(true)
    {
        bail!("base binary is not a valid PE file (missing PE signature)");
    }

    // The Optional Header begins right after the 4-byte PE signature and the
    // 20-byte COFF File Header. Its first field is the 2-byte Magic that
    // tells us whether this is PE32 (0x10b) or PE32+ (0x20b).
    let opt_header = pe_off + 24;
    if bin.len() < opt_header + 2 {
        bail!("PE optional header truncated");
    }
    let magic = u16::from_le_bytes(
        bin[opt_header..opt_header + 2]
            .try_into()
            .map_err(|_| anyhow!("PE magic slice error"))?,
    );
    if magic != 0x10b && magic != 0x20b {
        bail!("unknown PE optional header magic: 0x{magic:x}");
    }

    // Subsystem sits at offset 68 inside the Optional Header for both PE32
    // and PE32+. The 4-byte difference (BaseOfData in PE32 vs an 8-byte
    // ImageBase in PE32+) cancels out so the field lands at the same offset.
    let subsystem_off = opt_header + 68;
    if bin.len() < subsystem_off + 2 {
        bail!("PE subsystem field out of bounds");
    }
    bin[subsystem_off..subsystem_off + 2].copy_from_slice(&2u16.to_le_bytes());
    Ok(())
}

/// Create a macOS `.app` bundle at `output` (ensuring it ends with `.app`).
///
/// Layout:
///   MyApp.app/
///     Contents/
///       Info.plist
///       MacOS/
///         <app_name>   ← the packed Mach-O binary
///       Resources/
fn create_macos_app_bundle(
    base_bytes: Vec<u8>,
    payload: Vec<u8>,
    output: &Path,
    app_name: &str,
    identifier: &str,
    version: &str,
    product_name: &str,
) -> Result<PathBuf> {
    let app_bundle = ensure_app_extension(output);
    let contents = app_bundle.join("Contents");
    let macos_dir = contents.join("MacOS");
    let resources = contents.join("Resources");

    fs::create_dir_all(&macos_dir).with_context(|| format!("creating {}", macos_dir.display()))?;
    fs::create_dir_all(&resources)?;

    let binary_path = macos_dir.join(app_name);
    write_payload_into_exe(base_bytes, payload, &binary_path)?;

    let info_plist = generate_info_plist(app_name, identifier, version, product_name);
    fs::write(contents.join("Info.plist"), info_plist)?;

    Ok(app_bundle)
}

fn ensure_app_extension(path: &Path) -> PathBuf {
    let s = path.to_string_lossy();
    if s.ends_with(".app") {
        path.to_path_buf()
    } else {
        PathBuf::from(format!("{s}.app"))
    }
}

fn generate_info_plist(
    app_name: &str,
    identifier: &str,
    version: &str,
    product_name: &str,
) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleName</key>
    <string>{product_name}</string>
    <key>CFBundleDisplayName</key>
    <string>{product_name}</string>
    <key>CFBundleIdentifier</key>
    <string>{identifier}</string>
    <key>CFBundleExecutable</key>
    <string>{app_name}</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleInfoDictionaryVersion</key>
    <string>6.0</string>
    <key>CFBundleVersion</key>
    <string>{version}</string>
    <key>CFBundleShortVersionString</key>
    <string>{version}</string>
    <key>LSMinimumSystemVersion</key>
    <string>11.0</string>
    <key>NSHighResolutionCapable</key>
    <true/>
    <key>NSSupportsAutomaticGraphicsSwitching</key>
    <true/>
</dict>
</plist>"#
    )
}

/// Returns the raw bytes of `base` with any existing embedded payload stripped.
/// This lets `uzumaki pack` be run against a binary that is itself a packed
/// executable, or against a plain runtime binary.
fn read_base_exe_without_payload(base: &Path) -> Result<Vec<u8>> {
    let mut bytes =
        fs::read(base).with_context(|| format!("reading base binary {}", base.display()))?;
    if let Some(existing) = read_payload_from_exe(base)? {
        // Truncate to payload_start. We can recover it by re-opening via the
        // trailer, but simpler: search for the recorded start through the
        // deserializer already validated it. Parse the last 16 bytes again
        // here to get the offset.
        let _ = existing;
        let len = bytes.len();
        if len >= 16 {
            let payload_start = u64::from_le_bytes(
                bytes[len - 16..len - 8]
                    .try_into()
                    .map_err(|_| anyhow!("trailer slice error"))?,
            ) as usize;
            if payload_start <= len {
                bytes.truncate(payload_start);
            }
        }
    }
    Ok(bytes)
}
