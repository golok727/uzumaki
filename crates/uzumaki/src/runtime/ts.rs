use std::borrow::Cow;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;

use deno_ast::MediaType;
use deno_ast::ParseParams;
use deno_ast::SourceMapOption;
use deno_core::ModuleLoadOptions;
use deno_core::ModuleLoadReferrer;
use deno_core::ModuleLoadResponse;
use deno_core::ModuleLoader;
use deno_core::ModuleSource;
use deno_core::ModuleSourceCode;
use deno_core::ModuleSpecifier;
use deno_core::ModuleType;
use deno_core::ResolutionKind;
use deno_core::error::ModuleLoaderError;
use deno_core::futures::FutureExt;
use deno_core::resolve_import;
use deno_error::JsErrorBox;
use node_resolver::NodeResolution;
use node_resolver::NodeResolutionKind;
use node_resolver::ResolutionMode;

use super::resolver::{UzCjsTracker, UzNodeCodeTranslator, UzNodeResolver};

/// Try appending TypeScript extensions to a file URL when the path doesn't exist.
fn try_resolve_ts(url: &ModuleSpecifier) -> Option<ModuleSpecifier> {
    let path = url.to_file_path().ok()?;
    if path.is_file() {
        return Some(url.clone());
    }
    for ext in &[".ts", ".tsx", ".js", ".jsx", ".mjs", ".mts"] {
        let mut candidate = path.as_os_str().to_owned();
        candidate.push(ext);
        let candidate = std::path::PathBuf::from(candidate);
        if candidate.is_file() {
            return ModuleSpecifier::from_file_path(&candidate).ok();
        }
    }
    // Try index files in directory
    if path.is_dir() {
        for name in &["index.ts", "index.tsx", "index.js"] {
            let candidate = path.join(name);
            if candidate.is_file() {
                return ModuleSpecifier::from_file_path(&candidate).ok();
            }
        }
    }
    None
}

pub type SourceMapStore = Rc<RefCell<HashMap<String, Vec<u8>>>>;

pub struct TypescriptModuleLoader {
    pub source_maps: SourceMapStore,
    pub node_resolver: Arc<UzNodeResolver>,
    pub cjs_tracker: Arc<UzCjsTracker>,
    pub node_code_translator: Arc<UzNodeCodeTranslator>,
}

impl ModuleLoader for TypescriptModuleLoader {
    fn resolve(
        &self,
        specifier: &str,
        referrer: &str,
        _kind: ResolutionKind,
    ) -> Result<ModuleSpecifier, ModuleLoaderError> {
        if specifier == "uzumaki" {
            return ModuleSpecifier::parse("ext:uzumaki/runtime.js").map_err(JsErrorBox::from_err);
        }

        let referrer_url = ModuleSpecifier::parse(referrer).unwrap_or_else(|_| {
            deno_core::resolve_url_or_path(referrer, &std::env::current_dir().unwrap()).unwrap()
        });

        // Use node_resolver for everything — it handles bare specifiers,
        // relative imports with extension resolution, etc.
        let resolved = match self.node_resolver.resolve(
            specifier,
            &referrer_url,
            ResolutionMode::Import,
            NodeResolutionKind::Execution,
        ) {
            Ok(NodeResolution::Module(url_or_path)) => {
                url_or_path.into_url().map_err(JsErrorBox::from_err)?
            }
            Ok(NodeResolution::BuiltIn(name)) => {
                return ModuleSpecifier::parse(&format!("node:{name}"))
                    .map_err(JsErrorBox::from_err);
            }
            Err(_) => {
                // Fallback to standard URL resolution (ext: schemes, etc.)
                resolve_import(specifier, referrer).map_err(JsErrorBox::from_err)?
            }
        };

        // Node resolver doesn't know about .ts/.tsx — try TS extensions
        if resolved.scheme() == "file"
            && let Some(ts_resolved) = try_resolve_ts(&resolved)
        {
            return Ok(ts_resolved);
        }

        Ok(resolved)
    }

    fn load(
        &self,
        module_specifier: &ModuleSpecifier,
        _maybe_referrer: Option<&ModuleLoadReferrer>,
        _options: ModuleLoadOptions,
    ) -> ModuleLoadResponse {
        let path = match module_specifier.to_file_path() {
            Ok(p) => p,
            Err(_) => {
                return ModuleLoadResponse::Sync(Err(JsErrorBox::generic(
                    "Only file:// URLs are supported.",
                )));
            }
        };

        let media_type = MediaType::from_path(&path);

        // Check if this is a CJS module that needs translation to ESM
        let is_maybe_cjs = self
            .cjs_tracker
            .is_maybe_cjs(module_specifier, media_type)
            .unwrap_or(false);

        if is_maybe_cjs {
            let translator = self.node_code_translator.clone();
            let specifier = module_specifier.clone();
            return ModuleLoadResponse::Async(
                async move {
                    let code = std::fs::read_to_string(&path).map_err(JsErrorBox::from_err)?;
                    let translated = translator
                        .translate_cjs_to_esm(&specifier, Some(Cow::Owned(code)))
                        .await
                        .map_err(JsErrorBox::from_err)?;
                    Ok(ModuleSource::new(
                        ModuleType::JavaScript,
                        ModuleSourceCode::String(translated.into_owned().into()),
                        &specifier,
                        None,
                    ))
                }
                .boxed_local(),
            );
        }

        // ESM path — existing sync logic
        let source_maps = self.source_maps.clone();
        fn load_esm(
            source_maps: SourceMapStore,
            module_specifier: &ModuleSpecifier,
            path: std::path::PathBuf,
            media_type: MediaType,
        ) -> Result<ModuleSource, ModuleLoaderError> {
            let (module_type, should_transpile) = match media_type {
                MediaType::JavaScript | MediaType::Mjs | MediaType::Cjs => {
                    (ModuleType::JavaScript, false)
                }
                MediaType::Jsx => (ModuleType::JavaScript, true),
                MediaType::TypeScript
                | MediaType::Mts
                | MediaType::Cts
                | MediaType::Dts
                | MediaType::Dmts
                | MediaType::Dcts
                | MediaType::Tsx => (ModuleType::JavaScript, true),
                MediaType::Json => (ModuleType::Json, false),
                _ => {
                    return Err(JsErrorBox::generic(format!(
                        "Unknown extension {:?}",
                        path.extension()
                    )));
                }
            };

            let code = std::fs::read_to_string(&path).map_err(JsErrorBox::from_err)?;
            let code = if should_transpile {
                let parsed = deno_ast::parse_module(ParseParams {
                    specifier: module_specifier.clone(),
                    text: code.into(),
                    media_type,
                    capture_tokens: false,
                    scope_analysis: false,
                    maybe_syntax: None,
                })
                .map_err(JsErrorBox::from_err)?;
                let res = parsed
                    .transpile(
                        &deno_ast::TranspileOptions {
                            imports_not_used_as_values: deno_ast::ImportsNotUsedAsValues::Remove,
                            decorators: deno_ast::DecoratorsTranspileOption::Ecma,
                            jsx: Some(deno_ast::JsxRuntime::Automatic(
                                deno_ast::JsxAutomaticOptions {
                                    development: std::env::var("NODE_ENV")
                                        .map(|v| v != "production")
                                        .unwrap_or(true),
                                    import_source: Some("uzumaki-ui/react".to_string()),
                                },
                            )),
                            ..Default::default()
                        },
                        &deno_ast::TranspileModuleOptions { module_kind: None },
                        &deno_ast::EmitOptions {
                            source_map: SourceMapOption::Separate,
                            inline_sources: true,
                            ..Default::default()
                        },
                    )
                    .map_err(JsErrorBox::from_err)?;
                let res = res.into_source();
                let source_map = res.source_map.unwrap().into_bytes();
                source_maps
                    .borrow_mut()
                    .insert(module_specifier.to_string(), source_map);
                res.text
            } else {
                code
            };
            Ok(ModuleSource::new(
                module_type,
                ModuleSourceCode::String(code.into()),
                module_specifier,
                None,
            ))
        }

        ModuleLoadResponse::Sync(load_esm(source_maps, module_specifier, path, media_type))
    }

    fn get_source_map(&self, specifier: &str) -> Option<Cow<'_, [u8]>> {
        self.source_maps
            .borrow()
            .get(specifier)
            .map(|v: &Vec<u8>| Cow::Owned(v.clone()))
    }
}
