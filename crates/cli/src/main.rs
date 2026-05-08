pub mod cli;
pub mod init;
pub mod standalone;

use uzumaki_runtime::{AppConfig, Application, HeadlessApp};

use crate::standalone::LaunchMode;

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
    // always run it, ignoring any CLI args.
    match standalone::detect_and_prepare() {
        Ok(Some(config)) => {
            run_app(config);
            return;
        }
        Ok(None) => {
            // not standalone exe, fall back to cli
        }
        Err(err) => {
            eprintln!("uzumaki: failed to read embedded standalone payload: {err}");
            std::process::exit(1);
        }
    }

    // Not a standalone executable
    match cli::run_cli() {
        Ok(Some(mode)) => match mode {
            LaunchMode::Dev { config } | LaunchMode::Standalone { config, .. } => {
                run_app(config);
            }
            LaunchMode::Headless { config } => {
                run_headless(config);
            }
        },
        Ok(None) => {} // Command handled (build/pack/update) or help printed
        Err(err) => {
            eprintln!("\x1b[1;31merror:\x1b[0m {err:#}");
            std::process::exit(1);
        }
    }
}

fn run_headless(config: AppConfig) {
    let mut app = HeadlessApp::new(UZUMAKI_SNAPSHOT, config).expect("error creating headless app");

    app.run().expect("error running headless app");
}

fn run_app(config: AppConfig) {
    let mut app =
        Application::new_with_root(UZUMAKI_SNAPSHOT, config).expect("error creating application");

    app.run().expect("error running application");
}
