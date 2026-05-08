use std::path::Path;
use std::rc::Rc;
use std::sync::Arc;

use anyhow::Result;
use deno_resolver::npm::{
    ByonmNpmResolverCreateOptions, CreateInNpmPkgCheckerOptions, DenoInNpmPackageChecker,
    NpmResolver, NpmResolverCreateOptions,
};
use deno_runtime::BootstrapOptions;
use deno_runtime::deno_fs::FileSystem;
use deno_runtime::deno_node::NodeExtInitServices;
use deno_runtime::deno_permissions::PermissionsContainer;
use deno_runtime::deno_web::{BlobStore, InMemoryBroadcastChannel};
use deno_runtime::worker::{MainWorker, WorkerOptions, WorkerServiceOptions};
use node_resolver::analyze::{CjsModuleExportAnalyzer, NodeCodeTranslator, NodeCodeTranslatorMode};
use node_resolver::cache::NodeResolutionSys;

use crate::runtime;
use crate::runtime::module_loader::{UzCjsCodeAnalyzer, UzRequireLoader};
use crate::runtime::resolver::UzCjsTracker;
use crate::runtime::sys::UzSys;

pub struct WorkerBuildOptions<'a> {
    pub entry: &'a Path,
    pub app_root: &'a Path,
    pub args: Vec<String>,
    pub headless: bool,
    pub jsx_import_source: Option<String>,
    pub extensions: Vec<deno_core::Extension>,
    pub startup_snapshot: Option<&'static [u8]>,
}

/// Bootstraps a Deno worker. The resolver chain is shared between windowed
/// and headless apps; only the registered extensions and module-loader flags
/// differ. The caller must have an active Tokio runtime entered before
/// invoking this — `MainWorker::bootstrap_from_options` registers async
/// services that require one.
pub fn create_worker(opts: WorkerBuildOptions<'_>) -> Result<MainWorker> {
    let sys = sys_traits::impls::RealSys;

    // --- BYONM node resolution ---
    let root_node_modules = opts.app_root.join("node_modules");
    let has_node_modules_dir = root_node_modules.is_dir();
    let pkg_json_resolver: node_resolver::PackageJsonResolverRc<UzSys> =
        Arc::new(node_resolver::PackageJsonResolver::new(sys.clone(), None));

    let in_npm_pkg_checker = DenoInNpmPackageChecker::new(CreateInNpmPkgCheckerOptions::Byonm);

    let npm_resolver = NpmResolver::<UzSys>::new(NpmResolverCreateOptions::Byonm(
        ByonmNpmResolverCreateOptions {
            root_node_modules_dir: Some(root_node_modules),
            search_stop_dir: None,
            sys: NodeResolutionSys::new(sys.clone(), None),
            pkg_json_resolver: pkg_json_resolver.clone(),
        },
    ));

    let cjs_tracker = Arc::new(UzCjsTracker::new(
        in_npm_pkg_checker.clone(),
        pkg_json_resolver.clone(),
        deno_resolver::cjs::IsCjsResolutionMode::ImplicitTypeCommonJs,
        vec![],
    ));

    let node_resolver = Arc::new(node_resolver::NodeResolver::new(
        in_npm_pkg_checker.clone(),
        node_resolver::DenoIsBuiltInNodeModuleChecker,
        npm_resolver.clone(),
        pkg_json_resolver.clone(),
        NodeResolutionSys::new(sys.clone(), None),
        node_resolver::NodeResolverOptions::default(),
    ));

    let cjs_code_analyzer = UzCjsCodeAnalyzer {
        cjs_tracker: cjs_tracker.clone(),
    };
    let cjs_module_export_analyzer = Arc::new(CjsModuleExportAnalyzer::new(
        cjs_code_analyzer,
        in_npm_pkg_checker.clone(),
        node_resolver.clone(),
        npm_resolver.clone(),
        pkg_json_resolver.clone(),
        sys.clone(),
    ));
    let node_code_translator = Arc::new(NodeCodeTranslator::new(
        cjs_module_export_analyzer,
        NodeCodeTranslatorMode::ModuleLoader,
    ));

    let fs: Arc<dyn FileSystem> = Arc::new(deno_runtime::deno_fs::RealFs);

    let descriptor_parser =
        Arc::new(deno_runtime::permissions::RuntimePermissionDescriptorParser::new(sys.clone()));

    let main_module = deno_core::resolve_path(opts.entry.to_str().unwrap(), opts.app_root)?;

    let services = WorkerServiceOptions {
        blob_store: Arc::new(BlobStore::default()),
        broadcast_channel: InMemoryBroadcastChannel::default(),
        deno_rt_native_addon_loader: None,
        feature_checker: Arc::new(deno_runtime::FeatureChecker::default()),
        fs: fs.clone(),
        module_loader: Rc::new(runtime::ts::TypescriptModuleLoader {
            source_maps: runtime::ts::SourceMapStore::default(),
            node_resolver: node_resolver.clone(),
            cjs_tracker: cjs_tracker.clone(),
            node_code_translator,
            headless: opts.headless,
            jsx_import_source: opts.jsx_import_source,
        }),
        node_services: Some(NodeExtInitServices {
            node_require_loader: Rc::new(UzRequireLoader {
                cjs_tracker: cjs_tracker.clone(),
            }),
            node_resolver,
            pkg_json_resolver,
            sys: sys.clone(),
        }),
        npm_process_state_provider: None,
        permissions: PermissionsContainer::allow_all(descriptor_parser),
        root_cert_store_provider: None,
        fetch_dns_resolver: Default::default(),
        shared_array_buffer_store: None,
        compiled_wasm_module_store: None,
        v8_code_cache: None,
        bundle_provider: None,
    };

    let options = WorkerOptions {
        extensions: opts.extensions,
        startup_snapshot: opts.startup_snapshot,
        skip_op_registration: false,
        bootstrap: BootstrapOptions {
            args: opts.args,
            has_node_modules_dir,
            mode: deno_runtime::WorkerExecutionMode::None,
            ..Default::default()
        },
        ..Default::default()
    };

    let worker = MainWorker::bootstrap_from_options(&main_module, services, options);

    Ok(worker)
}
