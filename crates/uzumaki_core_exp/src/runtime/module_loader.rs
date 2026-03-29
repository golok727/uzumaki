use super::resolver::UzCjsTracker;
use std::borrow::Cow;
use std::path::Path;
use std::sync::Arc;

use deno_ast::MediaType;
use deno_error::JsErrorBox;
use deno_runtime::deno_permissions::PermissionsContainer;
use node_resolver::analyze::{CjsAnalysis, CjsAnalysisExports, CjsCodeAnalyzer, EsmAnalysisMode};

pub struct UzCjsCodeAnalyzer {
    pub cjs_tracker: Arc<UzCjsTracker>,
}

#[async_trait::async_trait(?Send)]
impl CjsCodeAnalyzer for UzCjsCodeAnalyzer {
    async fn analyze_cjs<'a>(
        &self,
        specifier: &deno_core::url::Url,
        source: Option<Cow<'a, str>>,
        _esm_analysis_mode: EsmAnalysisMode,
    ) -> Result<CjsAnalysis<'a>, JsErrorBox> {
        let source = match source {
            Some(s) => s,
            None => {
                let path = specifier.to_file_path().map_err(|_| {
                    JsErrorBox::generic(format!("Cannot convert to file path: {}", specifier))
                })?;
                Cow::Owned(std::fs::read_to_string(&path).map_err(JsErrorBox::from_err)?)
            }
        };

        let media_type = MediaType::from_specifier(specifier);
        if media_type == MediaType::Json {
            return Ok(CjsAnalysis::Cjs(CjsAnalysisExports {
                exports: vec![],
                reexports: vec![],
            }));
        }

        let is_maybe_cjs = self
            .cjs_tracker
            .is_maybe_cjs(specifier, media_type)
            .map_err(JsErrorBox::from_err)?;

        if !is_maybe_cjs {
            return Ok(CjsAnalysis::Esm(source, None));
        }

        let parsed = deno_ast::parse_program(deno_ast::ParseParams {
            specifier: specifier.clone(),
            text: source.to_string().into(),
            media_type,
            capture_tokens: true,
            scope_analysis: false,
            maybe_syntax: None,
        })
        .map_err(JsErrorBox::from_err)?;

        if parsed.compute_is_script() {
            let analysis = parsed.analyze_cjs();
            Ok(CjsAnalysis::Cjs(CjsAnalysisExports {
                exports: analysis.exports,
                reexports: analysis.reexports,
            }))
        } else {
            Ok(CjsAnalysis::Esm(source, None))
        }
    }
}

pub struct UzRequireLoader {
    pub cjs_tracker: Arc<UzCjsTracker>,
}

impl deno_runtime::deno_node::NodeRequireLoader for UzRequireLoader {
    fn ensure_read_permission<'a>(
        &self,
        _permissions: &mut PermissionsContainer,
        path: Cow<'a, Path>,
    ) -> Result<Cow<'a, Path>, deno_error::JsErrorBox> {
        Ok(path)
    }

    fn load_text_file_lossy(
        &self,
        path: &Path,
    ) -> Result<deno_core::FastString, deno_error::JsErrorBox> {
        let text = std::fs::read_to_string(path).map_err(deno_error::JsErrorBox::from_err)?;
        Ok(text.into())
    }

    fn is_maybe_cjs(
        &self,
        specifier: &deno_core::url::Url,
    ) -> Result<bool, node_resolver::errors::PackageJsonLoadError> {
        let media_type = MediaType::from_specifier(specifier);
        self.cjs_tracker.is_maybe_cjs(specifier, media_type)
    }
}
