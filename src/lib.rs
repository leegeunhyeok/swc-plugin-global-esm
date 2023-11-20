mod module_collector;
mod utils;

use module_collector::{ExportModule, ImportModule, ModuleCollector, ModuleType};
use serde::Deserialize;
use std::collections::HashMap;
use swc_core::{
    atoms::js_word,
    common::{Span, DUMMY_SP},
    ecma::{
        ast::*,
        visit::{as_folder, noop_visit_mut_type, FoldWith, VisitMut, VisitMutWith},
    },
    plugin::{
        metadata::TransformPluginMetadataContextKind, plugin_transform,
        proxies::TransformPluginProgramMetadata,
    },
};
use utils::{
    call_expr, decl_var_and_assign_stmt, fn_arg, ident, ident_expr, obj_member_expr, str_lit_expr,
};

const GLOBAL: &str = "global";
const MODULE: &str = "__modules";
const MODULE_IMPORT_METHOD_NAME: &str = "import";
const MODULE_EXPORT_METHOD_NAME: &str = "export";

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct GlobalEsmModuleOptions {
    runtime_module: Option<bool>,
    import_paths: Option<HashMap<String, String>>,
}

pub struct GlobalEsmModule {
    module_name: String,
    runtime_module: bool,
    import_paths: Option<HashMap<String, String>>,
}

impl GlobalEsmModule {
    /// Find actual module path from `import_paths`
    fn to_actual_path(&mut self, module_src: String) -> String {
        if let Some(actual_path) = self
            .import_paths
            .as_ref()
            .and_then(|import_paths| import_paths.get(&module_src))
        {
            return actual_path.clone();
        }
        module_src
    }

    /// Returns an expression that import module from global.
    ///
    /// eg. `global.__modules.import(module_src)`
    fn get_global_import_expr(&mut self, module_src: String) -> Expr {
        call_expr(
            obj_member_expr(
                obj_member_expr(ident_expr(js_word!(GLOBAL)), ident(js_word!(MODULE))),
                Ident::new(js_word!(MODULE_IMPORT_METHOD_NAME), DUMMY_SP),
            ),
            vec![fn_arg(str_lit_expr(self.to_actual_path(module_src)))],
        )
    }

    /// Returns an expression that export module to global.
    ///
    /// eg. `global.__modules.export(module_name, expr)`
    fn get_global_export_expr(&mut self, export_expr: Expr) -> Expr {
        call_expr(
            obj_member_expr(
                obj_member_expr(ident_expr(js_word!(GLOBAL)), ident(js_word!(MODULE))),
                Ident::new(js_word!(MODULE_EXPORT_METHOD_NAME), DUMMY_SP),
            ),
            vec![
                fn_arg(str_lit_expr(self.module_name.to_owned())),
                fn_arg(export_expr),
            ],
        )
    }

    /// Returns a statement that import default value from global.
    ///
    /// eg. `const ident = global.__modules.import(module_src).default`
    fn default_import_stmt(&mut self, module_src: String, span: Span, ident: Ident) -> Stmt {
        decl_var_and_assign_stmt(
            ident,
            span,
            obj_member_expr(
                self.get_global_import_expr(module_src),
                Ident::new("default".into(), DUMMY_SP),
            ),
        )
    }

    /// Returns a statement that import named value from global.
    ///
    /// eg. `const ident = global.__modules.import(module_src).ident`
    fn named_import_stmt(&mut self, module_src: String, span: Span, ident: Ident) -> Stmt {
        decl_var_and_assign_stmt(
            ident.clone(),
            span,
            obj_member_expr(
                self.get_global_import_expr(module_src),
                Ident::new(ident.sym, DUMMY_SP),
            ),
        )
    }

    /// Returns a statement that import namespaced value from global.
    ///
    /// eg. `const ident = global.__modules.import(module_src)`
    fn namespace_import_stmt(&mut self, module_src: String, span: Span, ident: Ident) -> Stmt {
        decl_var_and_assign_stmt(ident.clone(), span, self.get_global_import_expr(module_src))
    }

    /// Returns an exports object literal expression.
    ///
    /// eg. `{ default: value, named: value }`
    fn get_exports_obj_expr(&mut self, exports: Vec<ExportModule>) -> Expr {
        if exports.len() == 0 {
            return Expr::Lit(Lit::Null(Null { span: DUMMY_SP }));
        }

        let mut export_props = Vec::new();
        exports.into_iter().for_each(
            |ExportModule {
                 ident,
                 as_ident,
                 module_type,
             }| {
                export_props.push(match module_type {
                    ModuleType::Default => {
                        PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
                            key: PropName::Ident(Ident::new(js_word!("default"), DUMMY_SP)),
                            value: Box::new(Expr::Ident(ident)),
                        })))
                    }
                    ModuleType::Named => {
                        if let Some(renamed_ident) = as_ident {
                            PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
                                key: PropName::Ident(Ident::new(renamed_ident.sym, DUMMY_SP)),
                                value: Box::new(Expr::Ident(ident)),
                            })))
                        } else {
                            PropOrSpread::Prop(Box::new(Prop::Shorthand(ident)))
                        }
                    }
                    ModuleType::NamespaceOrAll => PropOrSpread::Spread(SpreadElement {
                        dot3_token: DUMMY_SP,
                        expr: Box::new(Expr::Ident(ident)),
                    }),
                });
            },
        );

        Expr::Object(ObjectLit {
            span: DUMMY_SP,
            props: export_props,
        })
    }

    /// Returns an exports to global statement.
    ///
    /// eg: `global.__modules.export(module_name, exports_obj)`
    fn get_global_exports_stmt(&mut self, exports: Vec<ExportModule>) -> Stmt {
        let exports_obj = self.get_exports_obj_expr(exports);
        Stmt::Expr(ExprStmt {
            span: DUMMY_SP,
            expr: Box::new(self.get_global_export_expr(exports_obj)),
        })
    }
}

impl VisitMut for GlobalEsmModule {
    noop_visit_mut_type!();

    fn visit_mut_module(&mut self, module: &mut Module) {
        let mut collector = ModuleCollector::default(self.runtime_module);
        module.visit_mut_with(&mut collector);

        let ModuleCollector {
            imports, exports, ..
        } = collector;
        let is_esm = imports.len() + exports.len() > 0;

        // Imports
        imports.into_iter().enumerate().for_each(
            |(
                index,
                ImportModule {
                    span,
                    ident,
                    module_src,
                    module_type,
                },
            )| match module_type {
                ModuleType::Default => {
                    module.body.insert(
                        index,
                        self.default_import_stmt(module_src, span, ident).into(),
                    );
                }
                ModuleType::Named => {
                    module.body.insert(
                        index,
                        self.named_import_stmt(module_src, span, ident).into(),
                    );
                }
                ModuleType::NamespaceOrAll => {
                    module.body.insert(
                        index,
                        self.namespace_import_stmt(module_src, span, ident).into(),
                    );
                }
            },
        );

        // Exports
        if is_esm {
            module
                .body
                .push(self.get_global_exports_stmt(exports).into());
        }
    }
}

#[plugin_transform]
pub fn global_esm_plugin(program: Program, metadata: TransformPluginProgramMetadata) -> Program {
    let config = serde_json::from_str::<GlobalEsmModuleOptions>(
        &metadata
            .get_transform_plugin_config()
            .expect("failed to get plugin config for swc-plugin-global-esm"),
    )
    .expect("invalid config for swc-plugin-global-esm");

    program.fold_with(&mut as_folder(GlobalEsmModule {
        module_name: metadata
            .get_context(&TransformPluginMetadataContextKind::Filename)
            .unwrap_or_default(),
        runtime_module: config.runtime_module.unwrap_or(false),
        import_paths: config.import_paths,
    }))
}

#[cfg(test)]
#[path = "./tests/esm_import.rs"]
mod esm_import;

#[cfg(test)]
#[path = "./tests/esm_export.rs"]
mod esm_export;

#[cfg(test)]
#[path = "./tests/bundle_time_module.rs"]
mod bundle_time_module;

#[cfg(test)]
#[path = "./tests/import_paths.rs"]
mod import_paths;
