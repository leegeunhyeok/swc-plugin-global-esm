use crate::utils::decl_var_and_assign_stmt;
use swc_core::{
    common::DUMMY_SP,
    ecma::{
        ast::*,
        utils::private_ident,
        visit::{noop_visit_mut_type, VisitMut},
    },
};
use tracing::debug;

#[derive(Debug)]
pub enum ModuleType {
    Default,
    Named,
    // `export { default as ... } from ...`
    DefaultAsNamed,
    // import: namespace, export: all
    NamespaceOrAll,
}

#[derive(Debug)]
pub struct ImportModule {
    pub ident: Ident,
    pub module_src: String,
    pub module_type: ModuleType,
}

impl ImportModule {
    fn default(ident: Ident, module_src: String) -> Self {
        ImportModule {
            ident,
            module_src,
            module_type: ModuleType::Default,
        }
    }

    fn named(ident: Ident, module_src: String) -> Self {
        ImportModule {
            ident,
            module_src,
            module_type: ModuleType::Named,
        }
    }

    fn namespace(ident: Ident, module_src: String) -> Self {
        ImportModule {
            ident,
            module_src,
            module_type: ModuleType::NamespaceOrAll,
        }
    }
}

#[derive(Debug)]
pub struct ExportModule {
    // `a` in `export { a as a_1 };`
    pub ident: Ident,
    // `a_1` in `export { a as a_1 };`
    pub as_ident: Option<Ident>,
    pub module_type: ModuleType,
}

impl ExportModule {
    fn default(ident: Ident) -> Self {
        ExportModule {
            ident,
            as_ident: None,
            module_type: ModuleType::Default,
        }
    }

    fn named(ident: Ident, as_ident: Option<Ident>) -> Self {
        ExportModule {
            ident,
            as_ident,
            module_type: ModuleType::Named,
        }
    }

    fn all(ident: Ident, as_ident: Option<Ident>) -> Self {
        ExportModule {
            ident,
            as_ident,
            module_type: ModuleType::NamespaceOrAll,
        }
    }
}

pub struct EsModuleCollector {
    pub imports: Vec<ImportModule>,
    pub exports: Vec<ExportModule>,
    runtime_module: bool,
}

impl EsModuleCollector {
    pub fn default(runtime_module: bool) -> Self {
        EsModuleCollector {
            runtime_module,
            imports: Vec::new(),
            exports: Vec::new(),
        }
    }

    /// Clone original module declare statements.
    ///
    /// noop for `runtime_module` is `true`.
    fn clone_module_decl_if_needed(
        &self,
        module_items: &mut Vec<ModuleItem>,
        module_decl: ModuleDecl,
    ) {
        if self.runtime_module {
            return;
        }
        module_items.push(module_decl.into());
    }

    /// Collect imports for emit global import statements.
    ///
    /// **Examples**
    ///
    /// - `import foo from 'src_1'`
    /// - `import { bar, baz as baz2 } from 'src_2'`
    ///
    /// ---
    ///
    /// - Identifiers: `foo`, `bar`, `baz` with original exported name `baz`.
    /// - Source: `src_1`, `src_2`.
    fn collect_import(&mut self, import_decl: &ImportDecl) {
        if !self.runtime_module {
            return;
        }
        import_decl.specifiers.iter().for_each(|import_spec| {
            let module_src = import_decl.src.value.to_string();
            match import_spec {
                ImportSpecifier::Default(ImportDefaultSpecifier { local, .. }) => {
                    debug!("default import: {:#?}", local.sym);
                    self.imports
                        .push(ImportModule::default(local.clone(), module_src));
                }
                ImportSpecifier::Named(ImportNamedSpecifier { local, .. }) => {
                    debug!("named import: {:#?}", local.sym);
                    self.imports
                        .push(ImportModule::named(local.clone(), module_src));
                }
                ImportSpecifier::Namespace(ImportStarAsSpecifier { local, .. }) => {
                    debug!("namespace import: {:#?}", local.sym);
                    self.imports
                        .push(ImportModule::namespace(local.clone(), module_src));
                }
            }
        });
    }

    /// Convert export declare statements to global module statements.
    ///
    /// **Examples**
    ///
    /// convert `export var foo = ...` to
    /// ```js
    /// // runtime_module: true
    /// var foo = ...;
    ///
    /// // runtime_module: false
    /// export var foo = ...;
    /// ```
    fn convert_export_decl(&mut self, export_decl: &ExportDecl) -> ModuleItem {
        match &export_decl.decl {
            Decl::Var(var_decl) => {
                if let Some(var_ident) = var_decl
                    .decls
                    .get(0)
                    .and_then(|var_declarator| var_declarator.name.as_ident())
                {
                    debug!("export decl var: {:#?}", var_ident.id.sym);
                    self.exports
                        .push(ExportModule::named(var_ident.id.clone(), None));
                }
            }
            Decl::Fn(FnDecl { ident, .. }) => {
                debug!("export decl fn: {:#?}", ident.sym);
                self.exports.push(ExportModule::named(ident.clone(), None));
            }
            Decl::Class(ClassDecl { ident, .. }) => {
                debug!("export decl class: {:#?}", ident.sym);
                self.exports.push(ExportModule::named(ident.clone(), None));
            }
            _ => {}
        }

        if self.runtime_module {
            Stmt::Decl(export_decl.decl.clone()).into()
        } else {
            ModuleItem::ModuleDecl(export_decl.clone().into())
        }
    }

    /// Convert export default declare statements to global module statements.
    ///
    /// **Examples**
    ///
    /// - Case 1: `export default function ident() { ... }` to
    ///   ```js
    ///   // runtime_module: true
    ///   function ident() { ... };
    ///
    ///   // runtime_module: false
    ///   function ident() { ... };
    ///   export default ident;
    ///   ```
    /// - Case 2: `export default function() { ... }` to
    ///   ```js
    ///   // runtime_module: true
    ///   const __export_default = function() { ... };
    ///
    ///   // runtime_module: false
    ///   const __export_default = function() { ... };
    ///   export default __export_default;
    ///   ```
    fn convert_export_default_decl(
        &mut self,
        export_default_decl: &ExportDefaultDecl,
    ) -> Vec<ModuleItem> {
        let (ident, stmt): (Ident, Stmt) = match &export_default_decl.decl {
            DefaultDecl::Fn(FnExpr {
                ident: Some(fn_ident),
                function,
                ..
            }) => {
                debug!("default export decl fn: {:#?}", fn_ident.sym);
                self.exports.push(ExportModule::default(fn_ident.clone()));
                (
                    fn_ident.clone(),
                    FnDecl {
                        ident: fn_ident.clone(),
                        function: function.clone(),
                        declare: false,
                    }
                    .into(),
                )
            }
            DefaultDecl::Class(ClassExpr {
                ident: Some(class_ident),
                class,
                ..
            }) => {
                debug!("default export decl class: {:#?}", class_ident.sym);
                self.exports
                    .push(ExportModule::default(class_ident.clone()));
                (
                    class_ident.clone(),
                    ClassDecl {
                        ident: class_ident.clone(),
                        class: class.clone(),
                        declare: false,
                    }
                    .into(),
                )
            }
            DefaultDecl::Fn(fn_expr) => {
                debug!("default export decl fn: <anonymous>");
                let ident = private_ident!("__export_default");
                let stmt = decl_var_and_assign_stmt(&ident, Expr::Fn(fn_expr.to_owned()));
                self.exports.push(ExportModule::default(ident.clone()));
                (ident, stmt)
            }
            DefaultDecl::Class(class_expr) => {
                debug!("default export decl class: <anonymous>");
                let ident = private_ident!("__export_default");
                let stmt = decl_var_and_assign_stmt(&ident, Expr::Class(class_expr.to_owned()));
                self.exports.push(ExportModule::default(ident.clone()));
                (ident, stmt)
            }
            _ => return vec![ModuleItem::ModuleDecl(export_default_decl.clone().into())],
        };

        return vec![
            stmt.into(),
            if self.runtime_module {
                ModuleItem::Stmt(Stmt::Empty(EmptyStmt { span: DUMMY_SP }))
            } else {
                ModuleItem::ModuleDecl(
                    ExportDefaultExpr {
                        span: DUMMY_SP,
                        expr: ident.into(),
                    }
                    .into(),
                )
            },
        ];
    }

    /// Convert export default expressions to global module statements.
    ///
    /// **Examples**
    ///
    /// `export default ident` to
    /// ```js
    /// // runtime_module: true
    /// const __export_default = ident;
    ///
    /// // runtime_module: false
    /// const __export_default = ident;
    /// export default __export_default;
    /// ```
    fn convert_export_default_expr(
        &mut self,
        export_default_expr: &ExportDefaultExpr,
    ) -> Vec<ModuleItem> {
        let ident = private_ident!("__export_default");
        let stmt = decl_var_and_assign_stmt(&ident, *export_default_expr.expr.to_owned());
        self.exports.push(ExportModule::default(ident.clone()));
        vec![
            stmt.into(),
            if self.runtime_module {
                ModuleItem::Stmt(Stmt::Empty(EmptyStmt { span: DUMMY_SP }))
            } else {
                ModuleItem::ModuleDecl(
                    ExportDefaultExpr {
                        span: DUMMY_SP,
                        expr: ident.into(),
                    }
                    .into(),
                )
            },
        ]
    }

    /// Collect modules from named exports.
    fn collect_named_exports(&mut self, named_export: &NamedExport) {
        debug!("named export {:#?}", named_export);
        match named_export {
            // without source
            // `export { ... };`
            NamedExport {
                src: None,
                specifiers,
                ..
            } => specifiers.iter().for_each(|export_spec| {
                if let ExportSpecifier::Named(ExportNamedSpecifier {
                    orig: ModuleExportName::Ident(orig_ident),
                    exported,
                    is_type_only: false,
                    ..
                }) = export_spec
                {
                    let as_ident = exported.as_ref().map(|export_name| match export_name {
                        ModuleExportName::Ident(as_ident) => as_ident.clone(),
                        ModuleExportName::Str(_) => unimplemented!(),
                    });
                    self.exports
                        .push(ExportModule::named(orig_ident.clone(), as_ident));
                }
            }),
            // with source (re-export)
            // Case 1: `export * as ... from '...';`
            // Case 2: `export { ... } from '...';`
            NamedExport {
                src: Some(module_src),
                specifiers,
                ..
            } => {
                if let Some(ExportSpecifier::Namespace(ExportNamespaceSpecifier {
                    name: ModuleExportName::Ident(module_ident),
                    ..
                })) = specifiers.get(0)
                {
                    // Case 1
                    let export_ident = private_ident!("__re_export");
                    self.imports.push(ImportModule::namespace(
                        export_ident.clone(),
                        module_src.value.to_string(),
                    ));
                    self.exports.push(ExportModule::named(
                        export_ident,
                        module_ident.clone().into(),
                    ));
                } else {
                    // Case 2
                    specifiers.iter().for_each(|import_spec| {
                        if let ExportSpecifier::Named(ExportNamedSpecifier {
                            orig: ModuleExportName::Ident(orig_ident),
                            exported,
                            ..
                        }) = import_spec
                        {
                            let is_default = orig_ident.sym == "default";
                            let target_ident = if is_default {
                                private_ident!("__default")
                            } else {
                                private_ident!(orig_ident.span, orig_ident.sym.clone())
                            };
                            self.imports.push(ImportModule {
                                ident: target_ident.clone(),
                                module_src: module_src.value.to_string(),
                                module_type: if is_default {
                                    ModuleType::DefaultAsNamed
                                } else {
                                    ModuleType::Named
                                },
                            });

                            match &exported {
                                Some(ModuleExportName::Ident(as_ident)) => self.exports.push(
                                    ExportModule::named(target_ident, as_ident.clone().into()),
                                ),
                                Some(ModuleExportName::Str(_)) => unimplemented!(),
                                None => self.exports.push(ExportModule::named(
                                    target_ident,
                                    orig_ident.clone().into(),
                                )),
                            }
                        }
                    });
                }
            }
        }
    }

    /// Collect modules from export all.
    fn collect_export_all(&mut self, export_all: &ExportAll) {
        debug!("export all {:#?}", export_all);
        let export_all_ident = private_ident!("__re_export_all");
        self.imports.push(ImportModule::namespace(
            export_all_ident.clone(),
            export_all.src.value.to_string(),
        ));
        self.exports
            .push(ExportModule::all(export_all_ident.clone(), None));
    }
}

impl VisitMut for EsModuleCollector {
    noop_visit_mut_type!();

    fn visit_mut_module(&mut self, module: &mut Module) {
        let mut module_body = Vec::with_capacity(module.body.len());
        for module_item in module.body.drain(..) {
            match module_item {
                ModuleItem::Stmt(stmt) => module_body.push(stmt.into()),
                ModuleItem::ModuleDecl(module_decl) => match &module_decl {
                    // Imports
                    ModuleDecl::Import(import_decl) => {
                        self.collect_import(import_decl);
                        self.clone_module_decl_if_needed(module_body.as_mut(), module_decl);
                    }
                    // Exports
                    // `export var ...`
                    // `export class ...`
                    // `export function ...`
                    ModuleDecl::ExportDecl(export_decl) => {
                        module_body.push(self.convert_export_decl(export_decl));
                    }
                    // `export default function ...`
                    // `export default class ...`
                    ModuleDecl::ExportDefaultDecl(export_default_decl) => {
                        module_body.extend(self.convert_export_default_decl(export_default_decl));
                    }
                    // `export default Identifier`
                    ModuleDecl::ExportDefaultExpr(export_default_expr) => {
                        module_body.extend(self.convert_export_default_expr(export_default_expr));
                    }
                    // Named export
                    // `export { ... }`
                    // `export { ident as ... }`
                    // `export { default as ... }`
                    //
                    // Export all
                    // `export * from ...`
                    ModuleDecl::ExportNamed(
                        named_export @ NamedExport {
                            type_only: false, ..
                        },
                    ) => {
                        self.collect_named_exports(named_export);
                        self.clone_module_decl_if_needed(module_body.as_mut(), module_decl);
                    }
                    ModuleDecl::ExportAll(
                        export_all @ ExportAll {
                            type_only: false, ..
                        },
                    ) => {
                        self.collect_export_all(export_all);
                        self.clone_module_decl_if_needed(module_body.as_mut(), module_decl);
                    }
                    _ => {
                        self.clone_module_decl_if_needed(module_body.as_mut(), module_decl);
                    }
                },
            };
        }

        // Remove empty statements for unused `;`.
        module_body.retain(|stmt| !matches!(stmt, ModuleItem::Stmt(Stmt::Empty(..))));
        module.body = module_body;
    }
}
