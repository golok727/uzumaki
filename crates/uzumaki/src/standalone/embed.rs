//! Platform-native embedding for the standalone payload.
//!
//! This is the production replacement for the v1 trailer-append approach in
//! [`super::format`]. The payload bytes (built by
//! [`super::format::serialize_payload_bytes`]) are stored in a real container
//! native to each executable format:
//!
//!   * Windows (PE):   an `RT_RCDATA` resource named [`SECTION_NAME`].
//!   * macOS  (Mach-O): a section in segment `__SUI` (libsui's fixed segment).
//!   * Linux  (ELF):    an appended note section.
//!
//! All three are handled by [`libsui`], which is the same library Deno uses
//! for `deno compile`. At runtime we use the platform-agnostic
//! [`libsui::find_section`] entry point to read it back from the running
//! executable.

use anyhow::{Context, Result, bail};
use std::fs::{self, File};
use std::path::Path;

/// Stable name used to identify our payload across platforms.
///
/// libsui maps this to the right per-format primitive:
///   * PE:    resource name (uppercased internally by libsui).
///   * Mach-O: section name inside the `__SUI` segment.
///   * ELF:   note section name.
pub const SECTION_NAME: &str = "uzumaki";

/// Embed `payload` into `base_bytes` using a real PE resource / Mach-O
/// section / ELF note section, and write the resulting executable to
/// `output`. The format is chosen automatically from the base binary's magic.
pub fn write_payload_into_exe(base_bytes: Vec<u8>, payload: Vec<u8>, output: &Path) -> Result<()> {
    if let Some(parent) = output.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;
    }

    let mut out = File::create(output).with_context(|| format!("creating {}", output.display()))?;

    if libsui::utils::is_pe(&base_bytes) {
        libsui::PortableExecutable::from(&base_bytes)
            .map_err(|e| anyhow::anyhow!("parsing base PE binary: {e}"))?
            .write_resource(SECTION_NAME, payload)
            .map_err(|e| anyhow::anyhow!("writing PE resource: {e}"))?
            .build(&mut out)
            .map_err(|e| anyhow::anyhow!("building PE output: {e}"))?;
    } else if libsui::utils::is_macho(&base_bytes) {
        libsui::Macho::from(base_bytes)
            .map_err(|e| anyhow::anyhow!("parsing base Mach-O binary: {e}"))?
            .write_section(SECTION_NAME, payload)
            .map_err(|e| anyhow::anyhow!("writing Mach-O section: {e}"))?
            .build_and_sign(&mut out)
            .map_err(|e| anyhow::anyhow!("building/signing Mach-O output: {e}"))?;
    } else if libsui::utils::is_elf(&base_bytes) {
        libsui::Elf::new(&base_bytes)
            .append(SECTION_NAME, &payload, &mut out)
            .map_err(|e| anyhow::anyhow!("appending ELF section: {e}"))?;
    } else {
        bail!("base binary is not a recognized executable format (PE/Mach-O/ELF)");
    }

    drop(out);

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(output)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(output, perms)?;
    }

    Ok(())
}

/// Read the embedded payload bytes from the *current* running executable.
/// Returns `Ok(None)` when no native section/resource is present (the boot
/// path can then fall back to the legacy v1 trailer reader).
pub fn read_payload_from_current_exe() -> Result<Option<Vec<u8>>> {
    match libsui::find_section(SECTION_NAME) {
        Ok(Some(bytes)) => Ok(Some(bytes.to_vec())),
        Ok(None) => Ok(None),
        Err(e) => Err(anyhow::Error::new(e).context("libsui::find_section failed")),
    }
}
