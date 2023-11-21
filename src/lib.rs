use std::collections::HashMap;

use serde::Deserialize;
use swc_core::{
    ecma::{ast::Program, visit::FoldWith},
    plugin::{
        metadata::TransformPluginMetadataContextKind, plugin_transform,
        proxies::TransformPluginProgramMetadata,
    },
};
use swc_global_esm::global_esm;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct GlobalEsmModuleOptions {
    runtime_module: Option<bool>,
    import_paths: Option<HashMap<String, String>>,
}

#[plugin_transform]
pub fn global_esm_plugin(program: Program, metadata: TransformPluginProgramMetadata) -> Program {
    let config = serde_json::from_str::<GlobalEsmModuleOptions>(
        &metadata
            .get_transform_plugin_config()
            .expect("failed to get plugin config for swc-plugin-global-esm"),
    )
    .expect("invalid config for swc-plugin-global-esm");

    program.fold_with(&mut global_esm(
        metadata
            .get_context(&TransformPluginMetadataContextKind::Filename)
            .unwrap_or_default(),
        config.runtime_module.unwrap_or(false),
        config.import_paths,
    ))
}
