fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    uzumaki_runtime::deno_runtime::deno_napi::print_linker_flags("uzumaki");

    let o = std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
    let cli_snapshot_path = o.join("UZUMAKI_SNAPSHOT.bin");

    let snapshot_options = uzumaki_runtime::deno_runtime::ops::bootstrap::SnapshotOptions {
        ts_version: uzumaki_runtime::TS_VERSION.to_string(),
        v8_version: uzumaki_runtime::deno_core::v8::VERSION_STRING,
        target: std::env::var("TARGET").unwrap(),
    };

    uzumaki_runtime::create_snapshot(cli_snapshot_path, snapshot_options);
}
