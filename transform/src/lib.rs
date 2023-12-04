mod constants;
mod module_collector;
mod utils;

use constants::{
    GLOBAL, MODULE, MODULE_EXPORT_ALL_METHOD_NAME, MODULE_EXPORT_METHOD_NAME,
    MODULE_IMPORT_METHOD_NAME, MODULE_IMPORT_WILDCARD_METHOD_NAME, MODULE_INIT_METHOD_NAME,
    MODULE_RESET_METHOD_NAME,
};
use module_collector::{ExportModule, ImportModule, ModuleCollector, ModuleType};
use regex::Regex;
use std::collections::{BTreeMap, HashMap};
use swc_core::{
    common::DUMMY_SP,
    ecma::{
        ast::*,
        utils::{private_ident, quote_ident, ExprFactory},
        visit::{as_folder, noop_visit_mut_type, Fold, VisitMut, VisitMutWith},
    },
};
use utils::{decl_var_and_assign_stmt, global_module_api_call_expr, obj_lit, obj_member_expr};

struct GlobalExports {
    export: Option<Stmt>,
    export_all: Option<Stmt>,
}

impl GlobalExports {
    fn new(
        module_name: &str,
        export_obj_expr: Option<Expr>,
        export_all_obj_expr: Option<Expr>,
    ) -> Self {
        GlobalExports {
            export: export_obj_expr.and_then(|expr| {
                global_module_api_call_expr(
                    MODULE_EXPORT_METHOD_NAME,
                    vec![module_name.as_arg(), expr.as_arg()],
                )
                .into_stmt()
                .into()
            }),
            export_all: export_all_obj_expr.and_then(|expr| {
                global_module_api_call_expr(
                    MODULE_EXPORT_ALL_METHOD_NAME,
                    vec![module_name.as_arg(), expr.as_arg()],
                )
                .into_stmt()
                .into()
            }),
        }
    }
}

struct ExportObjects {
    export: Option<Expr>,
    export_all: Option<Expr>,
}

impl ExportObjects {
    fn from_props(export_props: Vec<PropOrSpread>, export_all_props: Vec<PropOrSpread>) -> Self {
        ExportObjects {
            export: if !export_props.is_empty() {
                obj_lit(Some(export_props)).into()
            } else {
                None
            },
            export_all: if !export_all_props.is_empty() {
                obj_lit(Some(export_all_props)).into()
            } else {
                None
            },
        }
    }
}

pub struct GlobalEsmModule {
    module_name: String,
    runtime_module: bool,
    import_paths: Option<HashMap<String, String>>,
    import_idents: BTreeMap<String, Ident>,
    normalize_regex: Regex,
}

impl GlobalEsmModule {
    fn default(
        module_name: String,
        runtime_module: bool,
        import_paths: Option<HashMap<String, String>>,
    ) -> Self {
        GlobalEsmModule {
            module_name,
            runtime_module,
            import_paths,
            import_idents: BTreeMap::new(),
            normalize_regex: Regex::new(r"[^a-zA-Z0-9]").unwrap(),
        }
    }

    /// Find actual module path from `import_paths`
    fn to_actual_path(&self, module_src: String) -> String {
        if let Some(actual_path) = self
            .import_paths
            .as_ref()
            .and_then(|import_paths| import_paths.get(&module_src))
        {
            return actual_path.clone();
        }
        module_src
    }

    /// Returns an statement that import module from global and assign it.
    ///
    /// eg. `const __mod = global.__modules.import(module_src)`
    fn get_global_import_stmt(&self, ident: &Ident, module_src: &str) -> Stmt {
        decl_var_and_assign_stmt(
            ident,
            global_module_api_call_expr(
                MODULE_IMPORT_METHOD_NAME,
                vec![self.to_actual_path(String::from(module_src)).as_arg()],
            ),
        )
    }

    fn get_or_create_global_import_module_ident(&mut self, module_src: &String) -> &Ident {
        self.import_idents
            .entry(module_src.clone())
            .or_insert(private_ident!(self
                .normalize_regex
                .replace_all(format!("_{module_src}").as_str(), "_")
                .to_string()))
    }

    /// Create unique module identifier and returns a statement that import default value from global.
    ///
    /// eg. `const ident = {module_ident}.default`
    /// eg. `import ident from "module_src"`
    fn create_default_import_stmt(&mut self, module_src: &String, ident: &Ident) -> ModuleItem {
        if self.runtime_module {
            let module_ident = self.get_or_create_global_import_module_ident(&module_src);
            decl_var_and_assign_stmt(
                &ident,
                obj_member_expr(module_ident.to_owned().into(), quote_ident!("default")),
            )
            .into()
        } else {
            ModuleItem::ModuleDecl(ModuleDecl::Import(ImportDecl {
                span: DUMMY_SP,
                specifiers: vec![ImportDefaultSpecifier {
                    span: DUMMY_SP,
                    local: ident.to_owned(),
                }
                .into()],
                src: Str::from(module_src.to_owned()).into(),
                type_only: false,
                with: None,
            }))
        }
    }

    /// Create unique module identifier and returns a statement that import named value from global.
    ///
    /// eg. `const ident = {module_ident}.ident`
    /// eg. `import { ident } from "module_src"`
    fn create_named_import_stmt(&mut self, module_src: &String, ident: &Ident) -> ModuleItem {
        if self.runtime_module {
            let module_ident = self.get_or_create_global_import_module_ident(&module_src);
            decl_var_and_assign_stmt(
                &ident,
                obj_member_expr(
                    module_ident.to_owned().into(),
                    quote_ident!(ident.sym.to_owned()),
                ),
            )
            .into()
        } else {
            ModuleDecl::Import(ImportDecl {
                span: DUMMY_SP,
                specifiers: vec![ImportNamedSpecifier {
                    span: DUMMY_SP,
                    local: ident.to_owned(),
                    imported: None,
                    is_type_only: false,
                }
                .into()],
                src: Str::from(module_src.to_owned()).into(),
                type_only: false,
                with: None,
            })
            .into()
        }
    }

    /// Create unique module identifier and returns a statement that import namespaced value from global.
    ///
    /// eg. `const ident = global.__modules.importAll(module_src)`
    /// eg. `import * as ident from "module_src"`
    fn create_namespace_import_stmt(&mut self, module_src: &String, ident: &Ident) -> ModuleItem {
        if self.runtime_module {
            decl_var_and_assign_stmt(
                &ident,
                global_module_api_call_expr(
                    MODULE_IMPORT_WILDCARD_METHOD_NAME,
                    vec![Str::from(self.to_actual_path(module_src.to_owned())).as_arg()],
                ),
            )
            .into()
        } else {
            ModuleDecl::Import(ImportDecl {
                span: DUMMY_SP,
                src: Str::from(module_src.to_owned()).into(),
                type_only: false,
                with: None,
                specifiers: vec![ImportStarAsSpecifier {
                    span: DUMMY_SP,
                    local: ident.to_owned(),
                }
                .into()],
            })
            .into()
        }
    }

    /// Returns export and export all object literal expression.
    ///
    /// Export eg. `{ default: value, named: value }` or `{}`
    /// Export all eg. `{ ...all_export } or None`
    fn get_export_objects(&mut self, exports: Vec<ExportModule>) -> ExportObjects {
        let mut export_props = Vec::new();
        let mut export_all_props: Vec<PropOrSpread> = Vec::new();
        exports.into_iter().for_each(
            |ExportModule {
                 ident,
                 as_ident,
                 module_type,
             }| {
                match module_type {
                    ModuleType::Default | ModuleType::DefaultAsNamed => {
                        export_props.push(
                            Prop::KeyValue(KeyValueProp {
                                key: quote_ident!("default").into(),
                                value: ident.into(),
                            })
                            .into(),
                        );
                    }
                    ModuleType::Named => {
                        export_props.push(
                            if let Some(renamed_ident) =
                                as_ident.as_ref().filter(|&id| id.sym != ident.sym)
                            {
                                Prop::KeyValue(KeyValueProp {
                                    key: quote_ident!(renamed_ident.sym.as_str()).into(),
                                    value: ident.into(),
                                })
                                .into()
                            } else {
                                Prop::Shorthand(ident).into()
                            },
                        );
                    }
                    ModuleType::NamespaceOrAll => export_all_props.push(
                        SpreadElement {
                            dot3_token: DUMMY_SP,
                            expr: ident.into(),
                        }
                        .into(),
                    ),
                }
            },
        );

        ExportObjects::from_props(export_props, export_all_props)
    }

    /// Returns a statement that initialize the global module.
    ///
    /// eg. `global.__modules.init(module_name)`
    fn get_init_global_export_stmt(&mut self) -> Stmt {
        Stmt::Expr(ExprStmt {
            span: DUMMY_SP,
            expr: obj_member_expr(
                obj_member_expr(quote_ident!(GLOBAL).into(), quote_ident!(MODULE)),
                quote_ident!(MODULE_INIT_METHOD_NAME),
            )
            .as_call(DUMMY_SP, vec![Str::from(self.module_name.clone()).as_arg()])
            .into(),
        })
    }

    /// Returns a statement that reset the global module.
    ///
    /// eg. `global.__modules.reset(module_name)`
    fn get_reset_global_export_stmt(&mut self) -> Stmt {
        Stmt::Expr(ExprStmt {
            span: DUMMY_SP,
            expr: obj_member_expr(
                obj_member_expr(quote_ident!(GLOBAL).into(), quote_ident!(MODULE)),
                quote_ident!(MODULE_RESET_METHOD_NAME),
            )
            .as_call(DUMMY_SP, vec![Str::from(self.module_name.clone()).as_arg()])
            .into(),
        })
    }

    /// Returns an exports to global statement.
    ///
    /// eg: `global.__modules.export(module_name, exports_obj)`
    fn get_global_exports(&mut self, exports: Vec<ExportModule>) -> GlobalExports {
        let ExportObjects { export, export_all } = self.get_export_objects(exports);
        GlobalExports::new(&self.module_name, export, export_all)
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

        // Imports
        if !imports.is_empty() {
            module.body.splice(
                ..0,
                imports
                    .iter()
                    .map(
                        |ImportModule {
                             ident,
                             module_src,
                             module_type,
                         }| match module_type {
                            ModuleType::Default | ModuleType::DefaultAsNamed => {
                                self.create_default_import_stmt(module_src, ident)
                            }
                            ModuleType::Named => self.create_named_import_stmt(module_src, ident),
                            ModuleType::NamespaceOrAll => {
                                self.create_namespace_import_stmt(module_src, ident)
                            }
                        },
                    )
                    .collect::<Vec<_>>(),
            );

            module.body.splice(
                ..0,
                self.import_idents
                    .iter()
                    .map(|import_ident| {
                        self.get_global_import_stmt(import_ident.1, import_ident.0)
                            .into()
                    })
                    .collect::<Vec<_>>(),
            );
        }

        // Exports
        if !exports.is_empty() {
            module.body.push(self.get_init_global_export_stmt().into());
            let GlobalExports { export, export_all } = self.get_global_exports(exports);

            if let Some(export_stmt) = export {
                module.body.push(export_stmt.into());
            }

            if let Some(export_all_stmt) = export_all {
                module.body.push(export_all_stmt.into());
            }
        } else {
            module.body.push(self.get_reset_global_export_stmt().into());
        }
    }
}

pub fn global_esm(
    module_name: String,
    runtime_module: bool,
    import_paths: Option<HashMap<String, String>>,
) -> impl VisitMut + Fold {
    as_folder(GlobalEsmModule::default(
        module_name,
        runtime_module,
        import_paths,
    ))
}
