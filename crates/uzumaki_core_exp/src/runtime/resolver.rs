use deno_resolver::npm::{DenoInNpmPackageChecker, NpmResolver};
use node_resolver::DenoIsBuiltInNodeModuleChecker;

use super::module_loader::UzCjsCodeAnalyzer;
use super::sys::UzSys;

pub type UzCjsTracker = deno_resolver::cjs::CjsTracker<DenoInNpmPackageChecker, UzSys>;

pub type UzNodeResolver = node_resolver::NodeResolver<
    DenoInNpmPackageChecker,
    DenoIsBuiltInNodeModuleChecker,
    NpmResolver<UzSys>,
    UzSys,
>;

pub type UzCjsModuleExportAnalyzer = node_resolver::analyze::CjsModuleExportAnalyzer<
    UzCjsCodeAnalyzer,
    DenoInNpmPackageChecker,
    DenoIsBuiltInNodeModuleChecker,
    NpmResolver<UzSys>,
    UzSys,
>;

pub type UzNodeCodeTranslator = node_resolver::analyze::NodeCodeTranslator<
    UzCjsCodeAnalyzer,
    DenoInNpmPackageChecker,
    DenoIsBuiltInNodeModuleChecker,
    NpmResolver<UzSys>,
    UzSys,
>;
