pub mod cli;
pub mod init;
pub mod standalone;

use uzumaki_runtime::Application;

pub static UZUMAKI_SNAPSHOT: Option<&[u8]> = Some(include_bytes!(concat!(
    env!("OUT_DIR"),
    "/UZUMAKI_SNAPSHOT.bin"
)));

fn main() {
    #[cfg(target_os = "windows")]
    unsafe {
        std::env::set_var("WGPU_POWER_PREF", "high");
    }

    uzumaki_runtime::rustls::crypto::aws_lc_rs::default_provider()
        .install_default()
        .expect("failed to install rustls crypto provider");

    // Standalone-first: if the current executable carries an embedded payload,
    // always run it, ignoring any CLI args. This is what enables a
    // double-clicked `MyApp.exe` to "just work".
    match standalone::detect_and_prepare() {
        Ok(Some(mode)) => {
            run_launch_mode(mode);
            return;
        }
        Ok(None) => {}
        Err(err) => {
            eprintln!("uzumaki: failed to read embedded standalone payload: {err}");
            std::process::exit(1);
        }
    }

    // Not a standalone executable — use clap-based CLI.
    match cli::run_cli() {
        Ok(Some(mode)) => run_launch_mode(mode),
        Ok(None) => {} // Command handled (build/pack/update) or help printed
        Err(err) => {
            eprintln!("\x1b[1;31merror:\x1b[0m {err:#}");
            std::process::exit(1);
        }
    }
}

fn run_launch_mode(mode: standalone::LaunchMode) {
    let tokio_runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_io()
        .enable_time()
        .build()
        .expect("failed to create tokio runtime");
    let entry = mode.entry_path().to_path_buf();
    let app_root = mode.app_root().to_path_buf();
    let mut app = tokio_runtime.block_on(async {
        Application::new_with_root(entry, app_root, UZUMAKI_SNAPSHOT)
            .expect("error creating application")
    });
    app.tokio_runtime = Some(tokio_runtime);
    app.run().expect("error running application");
}
