pub mod module_loader;
pub mod resolver;
pub mod sys;

use module_loader::{UzCjsCodeAnalyzer, UzRequireLoader};

use sys::UzSys;

use anyhow::Result;
use deno_core::*;
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
use std::{collections::HashMap, path::PathBuf, rc::Rc, sync::Arc};
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ControlFlow, EventLoopProxy},
    window::WindowId,
};

use crate::resolver::UzCjsTracker;

mod ts;

pub static UZUMAKI_SNAPSHOT: Option<&[u8]> = Some(include_bytes!(concat!(
    env!("OUT_DIR"),
    "/UZUMAKI_SNAPSHOT.bin"
)));

fn main() {
    let tokio_runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("failed to create tokio runtime");

    let mut args = std::env::args();
    args.next();
    let entry_point = args.next().expect("no entry point provided");
    let cwd = std::env::current_dir().expect("error getting current directory");
    let entry_path = cwd.join(entry_point);
    let mut app = tokio_runtime
        .block_on(async { Application::new(entry_path).expect("error creating application") });
    app.tokio_runtime = Some(tokio_runtime);
    app.run().expect("error running application");
}

// for easy access from js
static WINDOW_ID_COUNTER: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

#[derive(Clone, Debug, deno_core::serde::Serialize, deno_core::serde::Deserialize)]
struct CreateWindowOptions {
    width: u32,
    height: u32,
    title: String,
    label: String, //  remove and alias this from js side  ?
}

#[derive(Debug, Clone)]
enum UserEvent {
    CreateWindow {
        js_id: u32,
        options: CreateWindowOptions,
    },
    Quit,
}

#[op2]
pub fn op_create_window(
    state: &mut OpState,
    #[serde] options: CreateWindowOptions,
) -> Result<u32, deno_error::JsErrorBox> {
    let proxy = state.borrow::<EventLoopProxy<UserEvent>>();
    let js_id = WINDOW_ID_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

    proxy
        .send_event(UserEvent::CreateWindow { js_id, options })
        .map_err(|_| {
            deno_error::JsErrorBox::new(
                "UzumakiInternalError",
                "cannot create window after application free",
            )
        })?;

    Ok(js_id)
}

#[op2(fast)]
pub fn op_request_quit(state: &mut OpState) -> Result<(), deno_error::JsErrorBox> {
    let proxy = state.borrow::<EventLoopProxy<UserEvent>>();
    proxy.send_event(UserEvent::Quit).map_err(|_| {
        deno_error::JsErrorBox::new("UzumakiInternalError", "error quitting window")
    })?;

    Ok(())
}

extension!(
  uzumaki,
  ops = [op_create_window, op_request_quit],
  esm_entry_point = "ext:uzumaki/00_init.js",
  esm = [ dir "core", "00_init.js" ],
);

struct Window {
    pub(crate) winit_window: Arc<winit::window::Window>,
}

impl Window {
    pub fn new(winit_window: Arc<winit::window::Window>) -> Result<Self> {
        Ok(Self { winit_window })
    }
}

struct Application {
    worker: MainWorker,
    windows: HashMap<WindowId, Arc<Window>>,
    main_file: PathBuf,
    event_loop: Option<winit::event_loop::EventLoop<UserEvent>>,
    module_loaded: bool,
    tokio_runtime: Option<tokio::runtime::Runtime>,
}

impl Application {
    pub fn new(main_file: impl Into<PathBuf>) -> Result<Self> {
        let main_file: PathBuf = main_file.into();
        let cwd = std::env::current_dir()?;
        let sys = sys_traits::impls::RealSys;

        // --- BYONM node resolution ---
        let root_node_modules = cwd.join("node_modules");
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

        // CJS-to-ESM translation pipeline
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

        let descriptor_parser = Arc::new(
            deno_runtime::permissions::RuntimePermissionDescriptorParser::new(sys.clone()),
        );

        let main_module = deno_core::resolve_path(main_file.to_str().unwrap(), &cwd)?;

        let services = WorkerServiceOptions {
            blob_store: Arc::new(BlobStore::default()),
            broadcast_channel: InMemoryBroadcastChannel::default(),
            deno_rt_native_addon_loader: None,
            feature_checker: Arc::new(deno_runtime::FeatureChecker::default()),
            fs: fs.clone(),
            module_loader: Rc::new(ts::TypescriptModuleLoader {
                source_maps: ts::SourceMapStore::default(),
                node_resolver: node_resolver.clone(),
                cjs_tracker: cjs_tracker.clone(),
                node_code_translator,
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
            extensions: vec![uzumaki::init()],
            startup_snapshot: UZUMAKI_SNAPSHOT,
            skip_op_registration: false,
            bootstrap: BootstrapOptions {
                args: vec![],
                mode: deno_runtime::WorkerExecutionMode::Run,
                ..Default::default()
            },
            ..Default::default()
        };

        let worker = MainWorker::bootstrap_from_options(&main_module, services, options);

        let event_loop: winit::event_loop::EventLoop<UserEvent> =
            winit::event_loop::EventLoop::with_user_event().build()?;

        {
            let state = worker.js_runtime.op_state();
            let mut borrow = state.borrow_mut();
            borrow.put(event_loop.create_proxy());
        }

        Ok(Self {
            worker,
            main_file,
            event_loop: Some(event_loop),
            windows: HashMap::new(),
            module_loaded: false,
            tokio_runtime: None,
        })
    }

    pub fn run(&mut self) -> Result<()> {
        let Some(event_loop) = self.event_loop.take() else {
            return Ok(());
        };
        event_loop.set_control_flow(ControlFlow::Poll);
        event_loop.run_app(self)?;
        Ok(())
    }

    fn tick_js(&mut self) {
        let rt = self.tokio_runtime.as_ref().unwrap();
        rt.block_on(async {
            tokio::select! {
                biased;
                result = self.worker.run_event_loop(false) => {
                    if let Err(e) = result {
                        eprintln!("JS error: {e}");
                    }
                }
                _ = tokio::task::yield_now() => {}
            }
        });
    }

    fn load_main_module(&mut self) {
        let specifier = deno_core::resolve_path(
            self.main_file.to_str().unwrap(),
            &std::env::current_dir().unwrap(),
        )
        .unwrap();

        let rt = self.tokio_runtime.as_ref().unwrap();
        rt.block_on(async {
            self.worker.execute_main_module(&specifier).await.unwrap();
        });
        self.tick_js();
    }
}

impl ApplicationHandler<UserEvent> for Application {
    fn resumed(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop) {
        if !self.module_loaded {
            self.module_loaded = true;
            self.load_main_module();
        }
    }

    fn about_to_wait(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop) {
        self.tick_js();
    }

    fn user_event(&mut self, event_loop: &winit::event_loop::ActiveEventLoop, event: UserEvent) {
        match event {
            UserEvent::CreateWindow {
                js_id: _,
                options: opts,
            } => {
                let attrs = winit::window::Window::default_attributes()
                    .with_title(&opts.title)
                    .with_inner_size(winit::dpi::LogicalSize::new(opts.width, opts.height));

                let winit_window = event_loop.create_window(attrs).unwrap();
                let id = winit_window.id();
                let window = Window::new(Arc::new(winit_window)).unwrap();
                self.windows.insert(id, Arc::new(window));
                println!("window created: {}", opts.title);
            }
            UserEvent::Quit => event_loop.exit(),
        }
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            _ => {}
        }
    }
}
