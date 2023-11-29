mod module_collector;
mod utils;

use module_collector::{ExportModule, ImportModule, ModuleCollector, ModuleType};
use std::collections::{BTreeMap, HashMap};
use swc_core::{
    common::DUMMY_SP,
    ecma::{
        ast::*,
        utils::{private_ident, quote_ident, ExprFactory},
        visit::{as_folder, noop_visit_mut_type, Fold, VisitMut, VisitMutWith},
    },
};
use utils::{decl_var_and_assign_stmt, obj_lit, obj_member_expr};

const GLOBAL: &str = "global";
const MODULE: &str = "__modules";
const MODULE_INIT_METHOD_NAME: &str = "init";
const MODULE_IMPORT_METHOD_NAME: &str = "import";
const MODULE_EXPORT_METHOD_NAME: &str = "export";

pub struct GlobalEsmModule {
    module_name: String,
    runtime_module: bool,
    import_paths: Option<HashMap<String, String>>,
    import_idents: BTreeMap<String, Ident>,
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
        }
    }

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

    /// Returns an statement that import module from global and assign it.
    ///
    /// eg. `const __mod = global.__modules.import(module_src)`
    fn get_global_import_stmt(&mut self, ident: &Ident, module_src: &String) -> Stmt {
        decl_var_and_assign_stmt(
            ident.clone(),
            ident.span,
            obj_member_expr(
                obj_member_expr(quote_ident!(GLOBAL).into(), quote_ident!(MODULE).into()),
                quote_ident!(MODULE_IMPORT_METHOD_NAME),
            )
            .as_call(
                DUMMY_SP,
                vec![Str::from(self.to_actual_path(module_src.clone())).as_arg()],
            ),
        )
    }

    fn get_or_create_global_import_module_ident(&mut self, module_src: &String) -> &Ident {
        self.import_idents
            .entry(module_src.clone())
            .or_insert(private_ident!("__module"))
    }

    /// Returns an expression that export module to global.
    ///
    /// eg. `global.__modules.export(module_name, expr)`
    fn get_global_export_expr(&mut self, export_expr: Expr, export_all_expr: Option<Expr>) -> Expr {
        let mut export_args = vec![
            Str::from(self.module_name.clone()).as_arg(),
            export_expr.as_arg(),
        ];

        if let Some(export_all) = export_all_expr {
            export_args.push(export_all.into());
        }

        obj_member_expr(
            obj_member_expr(quote_ident!(GLOBAL).into(), quote_ident!(MODULE).into()),
            quote_ident!(MODULE_EXPORT_METHOD_NAME),
        )
        .as_call(DUMMY_SP, export_args)
    }

    /// Returns a statement that import default value from global.
    ///
    /// eg. `const ident = {module_ident}.default`
    /// eg. `import ident from "module_src"`
    fn default_import_stmt(&mut self, module_src: String, ident: Ident) -> ModuleItem {
        if self.runtime_module {
            let module_ident = self.get_or_create_global_import_module_ident(&module_src);
            decl_var_and_assign_stmt(
                ident.clone(),
                ident.span,
                obj_member_expr(module_ident.clone().into(), quote_ident!("default")),
            )
            .into()
        } else {
            ModuleItem::ModuleDecl(ModuleDecl::Import(ImportDecl {
                span: DUMMY_SP,
                specifiers: vec![ImportDefaultSpecifier {
                    span: DUMMY_SP,
                    local: ident.clone(),
                }
                .into()],
                src: Str::from(module_src).into(),
                type_only: false,
                with: None,
            }))
        }
    }

    /// Returns a statement that import named value from global.
    ///
    /// eg. `const ident = {module_ident}.ident`
    /// eg. `import { ident } from "module_src"`
    fn named_import_stmt(&mut self, module_src: String, ident: Ident) -> ModuleItem {
        if self.runtime_module {
            let module_ident = self.get_or_create_global_import_module_ident(&module_src);
            decl_var_and_assign_stmt(
                ident.clone(),
                ident.span,
                obj_member_expr(module_ident.clone().into(), quote_ident!(ident.sym)),
            )
            .into()
        } else {
            ModuleDecl::Import(ImportDecl {
                span: DUMMY_SP,
                specifiers: vec![ImportNamedSpecifier {
                    span: DUMMY_SP,
                    local: ident.clone(),
                    imported: None,
                    is_type_only: false,
                }
                .into()],
                src: Str::from(module_src).into(),
                type_only: false,
                with: None,
            })
            .into()
        }
    }

    /// Returns a statement that import namespaced value from global.
    ///
    /// eg. `const ident = {module_ident}`
    /// eg. `import * as ident from "module_src"`
    fn namespace_import_stmt(&mut self, module_src: String, ident: Ident) -> ModuleItem {
        if self.runtime_module {
            decl_var_and_assign_stmt(
                ident.clone(),
                ident.span,
                self.get_or_create_global_import_module_ident(&module_src)
                    .clone()
                    .into(),
            )
            .into()
        } else {
            ModuleDecl::Import(ImportDecl {
                span: DUMMY_SP,
                src: Str::from(module_src).into(),
                type_only: false,
                with: None,
                specifiers: vec![ImportStarAsSpecifier {
                    span: DUMMY_SP,
                    local: ident.clone(),
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
    fn get_exports_obj_expr(&mut self, exports: Vec<ExportModule>) -> (Expr, Option<Expr>) {
        if exports.len() == 0 {
            return (obj_lit(None), None);
        }

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

        (
            obj_lit(Some(export_props)),
            if export_all_props.len() > 0 {
                Some(obj_lit(Some(export_all_props)))
            } else {
                None
            },
        )
    }

    fn get_init_global_export_stmt(&mut self) -> Stmt {
        Stmt::Expr(ExprStmt {
            span: DUMMY_SP,
            expr: obj_member_expr(
                obj_member_expr(quote_ident!(GLOBAL).into(), quote_ident!(MODULE).into()),
                quote_ident!(MODULE_INIT_METHOD_NAME),
            )
            .as_call(DUMMY_SP, vec![Str::from(self.module_name.clone()).as_arg()])
            .into(),
        })
    }

    /// Returns an exports to global statement.
    ///
    /// eg: `global.__modules.export(module_name, exports_obj)`
    fn get_global_export_stmt(&mut self, exports: Vec<ExportModule>) -> Stmt {
        let (export_obj, export_all_obj) = self.get_exports_obj_expr(exports);
        self.get_global_export_expr(export_obj, export_all_obj)
            .into_stmt()
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
                    ident,
                    module_src,
                    module_type,
                },
            )| match module_type {
                ModuleType::Default | ModuleType::DefaultAsNamed => {
                    module
                        .body
                        .insert(index, self.default_import_stmt(module_src, ident));
                }
                ModuleType::Named => {
                    module
                        .body
                        .insert(index, self.named_import_stmt(module_src, ident));
                }
                ModuleType::NamespaceOrAll => {
                    module
                        .body
                        .insert(index, self.namespace_import_stmt(module_src, ident));
                }
            },
        );

        // Exports
        if is_esm {
            module.body.push(self.get_init_global_export_stmt().into());
            module
                .body
                .push(self.get_global_export_stmt(exports).into());
        }

        self.import_idents
            .to_owned()
            .into_iter()
            .enumerate()
            .for_each(|(index, (module_src, ident))| {
                module.body.insert(
                    index,
                    self.get_global_import_stmt(&ident, &module_src).into(),
                );
            });
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
