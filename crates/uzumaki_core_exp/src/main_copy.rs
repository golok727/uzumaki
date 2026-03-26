use anyhow::Result;
use deno_core::*;
use deno_error::JsErrorBox;
use deno_node::NodeExtInitServices;
use deno_resolver::npm::{
    ByonmNpmResolverCreateOptions, CreateInNpmPkgCheckerOptions, DenoInNpmPackageChecker,
    NpmResolver, NpmResolverCreateOptions,
};
use futures::task::noop_waker;
use node_resolver::cache::NodeResolutionSys;
use std::borrow::Cow;
use std::path::Path;
use std::task::{Context, Poll};
use std::{collections::HashMap, path::PathBuf, rc::Rc, sync::Arc};
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ControlFlow, EventLoopProxy},
    window::WindowId,
};

mod ts;

type Sys = sys_traits::impls::RealSys;

// Minimal NodeRequireLoader — no permission checks, just reads files.
struct UzumakiRequireLoader;

impl deno_node::NodeRequireLoader for UzumakiRequireLoader {
    fn ensure_read_permission<'a>(
        &self,
        _permissions: &mut deno_permissions::PermissionsContainer,
        path: Cow<'a, Path>,
    ) -> Result<Cow<'a, Path>, JsErrorBox> {
        Ok(path)
    }

    fn load_text_file_lossy(&self, path: &Path) -> Result<deno_core::FastString, JsErrorBox> {
        let text = std::fs::read_to_string(path).map_err(JsErrorBox::from_err)?;
        Ok(text.into())
    }

    fn is_maybe_cjs(
        &self,
        specifier: &url::Url,
    ) -> Result<bool, node_resolver::errors::PackageJsonLoadError> {
        if let Ok(path) = specifier.to_file_path() {
            return Ok(path.extension().map_or(false, |e| e == "cjs"));
        }
        Ok(false)
    }
}

fn main() {
    let mut args = std::env::args();
    args.next();
    let entry_point = args.next().expect("no entry point provided");
    let cwd = std::env::current_dir().expect("error getting current directory");
    let entry_path = cwd.join(entry_point);
    let mut app = Application::new(entry_path).expect("error creating application");
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
fn op_create_window(
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
fn op_request_quit(state: &mut OpState) -> Result<(), deno_error::JsErrorBox> {
    let proxy = state.borrow::<EventLoopProxy<UserEvent>>();
    proxy.send_event(UserEvent::Quit).map_err(|_| {
        deno_error::JsErrorBox::new("UzumakiInternalError", "error quitting window")
    })?;

    Ok(())
}

// Stub op expected by deno_io's 12_io.js — TTY raw mode is irrelevant for a GUI runtime.
#[op2(fast)]
fn op_set_raw(_state: &mut OpState, _rid: u32, _mode: bool, _cbreak: bool) {}

extension!(
  uzumaki,
  ops = [op_create_window, op_request_quit, op_set_raw],
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
    js_runtime: JsRuntime,
    windows: HashMap<WindowId, Arc<Window>>,
    main_file: PathBuf,
    event_loop: Option<winit::event_loop::EventLoop<UserEvent>>,
    module_loaded: bool,
}

impl Application {
    pub fn new(main_file: impl Into<PathBuf>) -> Result<Self> {
        let main_file: PathBuf = main_file.into();
        let cwd = std::env::current_dir()?;
        let sys = sys_traits::impls::RealSys;

        // --- Set up BYONM node resolution ---
        let root_node_modules = cwd.join("node_modules");
        let pkg_json_resolver: node_resolver::PackageJsonResolverRc<Sys> =
            Rc::new(node_resolver::PackageJsonResolver::new(sys.clone(), None));

        let in_npm_pkg_checker = DenoInNpmPackageChecker::new(CreateInNpmPkgCheckerOptions::Byonm);

        let npm_resolver = NpmResolver::<Sys>::new(NpmResolverCreateOptions::Byonm(
            ByonmNpmResolverCreateOptions {
                root_node_modules_dir: Some(root_node_modules),
                search_stop_dir: None,
                sys: NodeResolutionSys::new(sys.clone(), None),
                pkg_json_resolver: pkg_json_resolver.clone(),
            },
        ));

        let node_resolver = Rc::new(deno_node::NodeResolver::new(
            in_npm_pkg_checker,
            node_resolver::DenoIsBuiltInNodeModuleChecker,
            npm_resolver,
            pkg_json_resolver.clone(),
            NodeResolutionSys::new(sys.clone(), None),
            node_resolver::NodeResolverOptions::default(),
        ));

        let fs: deno_fs::FileSystemRc = Rc::new(deno_fs::RealFs);

        let node_init_services = NodeExtInitServices {
            node_require_loader: Rc::new(UzumakiRequireLoader),
            node_resolver,
            pkg_json_resolver,
            sys: sys.clone(),
        };

        // --- Create JS runtime with lazy extensions ---
        let js_runtime = JsRuntime::new(RuntimeOptions {
            module_loader: Some(Rc::new(ts::TypescriptModuleLoader {
                source_maps: ts::SourceMapStore::default(),
            })),
            extensions: vec![
                deno_webidl::deno_webidl::init(),
                deno_web::deno_web::lazy_init(),
                deno_io::deno_io::lazy_init(),
                deno_fs::deno_fs::lazy_init(),
                deno_node::deno_node::lazy_init::<DenoInNpmPackageChecker, NpmResolver<Sys>, Sys>(),
                uzumaki::init(),
            ],
            extension_transpiler: Some(Rc::new(|specifier, source| {
                ts::transpile_extension(specifier, source)
            })),
            ..Default::default()
        });

        // Provide deferred state to lazy extensions
        js_runtime.lazy_init_extensions(vec![
            deno_web::deno_web::args(
                Arc::new(deno_web::BlobStore::default()),
                None,
                deno_web::InMemoryBroadcastChannel::default(),
            ),
            deno_io::deno_io::args(None),
            deno_fs::deno_fs::args(fs.clone()),
            deno_node::deno_node::args::<DenoInNpmPackageChecker, NpmResolver<Sys>, Sys>(
                Some(node_init_services),
                fs,
            ),
        ])?;

        let event_loop: winit::event_loop::EventLoop<UserEvent> =
            winit::event_loop::EventLoop::with_user_event().build()?;

        {
            let state = js_runtime.op_state();
            let mut borrow = state.borrow_mut();
            borrow.put(event_loop.create_proxy());
        }

        Ok(Self {
            js_runtime,
            main_file,
            event_loop: Some(event_loop),
            windows: HashMap::new(),
            module_loaded: false,
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
        let waker = noop_waker();
        let mut cx = Context::from_waker(&waker);
        match self.js_runtime.poll_event_loop(&mut cx, Default::default()) {
            Poll::Ready(Ok(_)) => {}
            Poll::Ready(Err(e)) => eprintln!("JS error: {e}"),
            Poll::Pending => {}
        }
    }

    fn load_main_module(&mut self) {
        let specifier = deno_core::resolve_path(
            self.main_file.to_str().unwrap(),
            &std::env::current_dir().unwrap(),
        )
        .unwrap();

        pollster::block_on(async {
            let mod_id = self
                .js_runtime
                .load_main_es_module(&specifier)
                .await
                .unwrap();
            let receiver = self.js_runtime.mod_evaluate(mod_id);
            self.js_runtime
                .run_event_loop(Default::default())
                .await
                .unwrap();
            receiver.await.unwrap();
        });
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
                js_id,
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
            _ => {}
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
