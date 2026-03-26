use deno_core::*;

extension!(
  uzumaki,
  esm_entry_point = "ext:uzumaki/00_init.js",
  esm = [ dir "core", "00_init.js" ],
);

pub static TS_VERSION: &str = "5.9.2";
fn main() {
    let o = std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
    let cli_snapshot_path = o.join("UZUMAKI_SNAPSHOT.bin");
    create_uz_snapshot(cli_snapshot_path);
}

fn create_uz_snapshot(snapshot_path: std::path::PathBuf) {
    use deno_runtime::ops::bootstrap::SnapshotOptions;

    let snapshot_options = SnapshotOptions {
        ts_version: TS_VERSION.to_string(),
        v8_version: deno_runtime::deno_core::v8::VERSION_STRING,
        target: std::env::var("TARGET").unwrap(),
    };

    deno_runtime::snapshot::create_runtime_snapshot(snapshot_path, snapshot_options, vec![]);
}
