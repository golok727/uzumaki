use anyhow::Result;
use deno_core::*;
use deno_resolver::npm::{
    ByonmNpmResolverCreateOptions, CreateInNpmPkgCheckerOptions, DenoInNpmPackageChecker,
    NpmResolver, NpmResolverCreateOptions,
};
use node_resolver::cache::NodeResolutionSys;
use std::task::Poll;
use std::{collections::HashMap, path::PathBuf, rc::Rc, sync::Arc};
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ControlFlow, EventLoopProxy},
    window::WindowId,
};

mod ts;

type Sys = sys_traits::impls::RealSys;
type UzumakiNodeResolver = node_resolver::NodeResolver<
    DenoInNpmPackageChecker,
    node_resolver::DenoIsBuiltInNodeModuleChecker,
    NpmResolver<Sys>,
    Sys,
>;

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

extension!(
  uzumaki,
  ops = [op_create_window, op_request_quit],
  esm_entry_point = "ext:uzumaki/00_init.js",
  esm = [ dir "core", "00_init.js", "timers.js" ],
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
    tokio_runtime: Option<tokio::runtime::Runtime>,
}

impl Application {
    pub fn new(main_file: impl Into<PathBuf>) -> Result<Self> {
        let main_file: PathBuf = main_file.into();
        let cwd = std::env::current_dir()?;
        let sys = sys_traits::impls::RealSys;

        // Set up BYONM node resolution
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

        let node_resolver: Rc<UzumakiNodeResolver> = Rc::new(node_resolver::NodeResolver::new(
            in_npm_pkg_checker,
            node_resolver::DenoIsBuiltInNodeModuleChecker,
            npm_resolver,
            pkg_json_resolver.clone(),
            NodeResolutionSys::new(sys.clone(), None),
            node_resolver::NodeResolverOptions::default(),
        ));

        let js_runtime = JsRuntime::new(RuntimeOptions {
            module_loader: Some(Rc::new(ts::TypescriptModuleLoader {
                source_maps: ts::SourceMapStore::default(),
                node_resolver: node_resolver.clone(),
            })),
            extensions: vec![uzumaki::init()],
            ..Default::default()
        });

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
                result = self.js_runtime.run_event_loop(Default::default()) => {
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
            let mod_id = self
                .js_runtime
                .load_main_es_module(&specifier)
                .await
                .unwrap();
            let _receiver = self.js_runtime.mod_evaluate(mod_id);
        });
        // Single tick to execute top-level code
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
