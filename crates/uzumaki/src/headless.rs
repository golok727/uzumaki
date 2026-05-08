use std::path::PathBuf;

use anyhow::Result;
use deno_runtime::worker::MainWorker;

use crate::AppConfig;
use crate::runtime::worker::{WorkerBuildOptions, create_worker};

/// A worker stripped of the windowing/GPU/clipboard machinery — used by
/// `uzumaki run` to execute scripts (build steps, codegen, tooling) without
/// spinning up the desktop app loop.
pub struct HeadlessApp {
    worker: MainWorker,
    main_file: PathBuf,
    app_root: PathBuf,
    tokio_runtime: tokio::runtime::Runtime,
}

impl HeadlessApp {
    pub fn new(startup_snapshot: Option<&'static [u8]>, config: AppConfig) -> Result<Self> {
        let tokio_runtime = tokio::runtime::Builder::new_current_thread()
            .enable_io()
            .enable_time()
            .build()
            .expect("failed to create tokio runtime");

        let main_file = config.entry.clone();
        let app_root = config.app_root.clone();

        let worker = {
            let _guard = tokio_runtime.enter();
            create_worker(WorkerBuildOptions {
                entry: &main_file,
                app_root: &app_root,
                args: config.args.clone(),
                headless: true,
                jsx_import_source: config.jsx_import_source.clone(),
                // Snapshot embeds the uzumaki extension so the same op table
                // must be present at runtime even though headless mode hides
                // the `uzumaki` module from JS — leaving it out triggers a
                // bounds error during op registration.
                extensions: vec![crate::uzumaki::init()],
                startup_snapshot,
            })?
        };

        {
            let op_state = worker.js_runtime.op_state();
            op_state.borrow_mut().put(config);
        }

        Ok(Self {
            worker,
            main_file,
            app_root,
            tokio_runtime,
        })
    }

    /// Executes the entry module and runs the JS event loop to completion.
    pub fn run(&mut self) -> Result<()> {
        let main_module =
            deno_core::resolve_path(self.main_file.to_str().unwrap(), &self.app_root)?;
        self.tokio_runtime.block_on(async {
            self.worker.execute_main_module(&main_module).await?;
            self.worker.run_event_loop(false).await?;
            Ok::<_, anyhow::Error>(())
        })?;
        Ok(())
    }
}
