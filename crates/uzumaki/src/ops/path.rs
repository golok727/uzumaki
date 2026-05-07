use std::path::{Path, PathBuf};

use deno_core::*;

use crate::app::AppConfig;

pub struct AppPath;

unsafe impl GarbageCollected for AppPath {
    fn trace(&self, _visitor: &mut deno_core::v8::cppgc::Visitor) {}

    fn get_name(&self) -> &'static std::ffi::CStr {
        c"CoreAppPath"
    }
}

#[op2]
#[allow(non_snake_case)]
impl AppPath {
    #[constructor]
    #[cppgc]
    pub fn new() -> AppPath {
        AppPath
    }
    /// Resolve a path under the bundled resource root. Dumb join — no I/O,
    /// no traversal sanitation. Caller is trusted (it's app code).
    #[string]
    pub fn resource(&self, state: &OpState, #[string] rel: &str) -> String {
        let paths = state.borrow::<AppConfig>();
        path_to_string(&paths.resource_root.join(normalize_rel(rel)))
    }

    #[getter]
    #[string]
    pub fn resourceDir(&self, state: &OpState) -> String {
        let paths = state.borrow::<AppConfig>();
        path_to_string(&paths.resource_root)
    }

    #[getter]
    #[string]
    pub fn identifier(&self, state: &OpState) -> String {
        let paths = state.borrow::<AppConfig>();
        paths.identifier.clone()
    }

    #[string]
    pub fn cacheDir(&self) -> Option<String> {
        dirs::cache_dir().as_deref().map(path_to_string)
    }

    #[string]
    pub fn dataDir(&self) -> Option<String> {
        dirs::data_dir().as_deref().map(path_to_string)
    }

    #[string]
    pub fn configDir(&self) -> Option<String> {
        dirs::config_dir().as_deref().map(path_to_string)
    }

    #[string]
    pub fn tempDir(&self) -> String {
        path_to_string(&std::env::temp_dir())
    }

    #[string]
    pub fn exeDir(&self) -> Option<String> {
        std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(Path::to_path_buf))
            .as_deref()
            .map(path_to_string)
    }

    #[string]
    pub fn homeDir(&self) -> Option<String> {
        dirs::home_dir().as_deref().map(path_to_string)
    }
}

fn normalize_rel(rel: &str) -> PathBuf {
    PathBuf::from(rel.replace(['\\', '/'], std::path::MAIN_SEPARATOR_STR))
}

fn path_to_string(p: &Path) -> String {
    p.to_string_lossy().into_owned()
}
