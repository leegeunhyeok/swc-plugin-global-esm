use crate::utils::is_invalid_module_decl;
use swc_core::{
    common::util::take::Take,
    ecma::{
        ast::*,
        utils::{private_ident, ExprFactory},
        visit::{noop_visit_mut_type, VisitMut, VisitMutWith},
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
    pub imported: Option<Ident>,
    pub module_src: String,
    pub module_type: ModuleType,
}

impl ImportModule {
    fn default(ident: Ident, imported: Option<Ident>, module_src: String) -> Self {
        ImportModule {
            ident,
            imported,
            module_src,
            module_type: ModuleType::Default,
        }
    }

    fn named(ident: Ident, imported: Option<Ident>, module_src: String) -> Self {
        ImportModule {
            ident,
            imported,
            module_src,
            module_type: ModuleType::Named,
        }
    }

    fn namespace(ident: Ident, imported: Option<Ident>, module_src: String) -> Self {
        ImportModule {
            ident,
            imported,
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
    runtime_module: bool,
    pub imports: Vec<ImportModule>,
    pub exports: Vec<ExportModule>,
}

impl EsModuleCollector {
    pub fn new(runtime_module: bool) -> Self {
        EsModuleCollector {
            runtime_module,
            imports: Vec::new(),
            exports: Vec::new(),
        }
    }

    /// Collect `ExportModule` from default export expressions.
    ///
    /// - `export default expr`
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
    fn collect_and_convert_export_default_expr(
        &mut self,
        export_default_expr: &mut ExportDefaultExpr,
    ) -> Stmt {
        debug!("export default expr {:#?}", export_default_expr);
        let ident = private_ident!("__export_default");
        self.exports.push(ExportModule::default(ident.clone()));
        export_default_expr
            .expr
            .clone()
            .into_var_decl(VarDeclKind::Const, ident.clone().into())
            .into()
    }

    /// Collect `ExportModule` default export with declare statements.
    ///
    /// - `export default function ...`
    /// - `export default class ...`
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
    ///   const __fn = function() { ... };
    ///
    ///   // runtime_module: false
    ///   const __fn = function() { ... };
    ///   export default __fn;
    ///   ```
    fn collect_and_convert_export_default_decl(
        &mut self,
        export_default_decl: &mut ExportDefaultDecl,
    ) -> Option<Stmt> {
        debug!("export default decl {:#?}", export_default_decl);

        match &mut export_default_decl.decl {
            DefaultDecl::Class(class_expr) => match class_expr {
                ClassExpr {
                    ident: Some(class_ident),
                    ..
                } => {
                    self.exports
                        .push(ExportModule::default(class_ident.clone()));
                    if self.runtime_module {
                        Some(Stmt::Decl(Decl::Class(
                            class_expr.clone().as_class_decl().unwrap(),
                        )))
                    } else {
                        None
                    }
                }
                ClassExpr { ident: None, .. } => {
                    let ident = private_ident!("__Class");
                    self.exports.push(ExportModule::default(ident.clone()));
                    class_expr.ident = Some(ident);
                    if self.runtime_module {
                        Some(Stmt::Decl(Decl::Class(
                            class_expr.clone().as_class_decl().unwrap(),
                        )))
                    } else {
                        None
                    }
                }
            },
            DefaultDecl::Fn(fn_expr) => match fn_expr {
                FnExpr {
                    ident: Some(fn_ident),
                    ..
                } => {
                    self.exports.push(ExportModule::default(fn_ident.clone()));
                    if self.runtime_module {
                        Some(Stmt::Decl(Decl::Fn(fn_expr.clone().as_fn_decl().unwrap())))
                    } else {
                        None
                    }
                }
                FnExpr { ident: None, .. } => {
                    let ident = private_ident!("__fn");
                    self.exports.push(ExportModule::default(ident.clone()));
                    fn_expr.ident = Some(ident);
                    if self.runtime_module {
                        Some(Stmt::Decl(Decl::Fn(fn_expr.clone().as_fn_decl().unwrap())))
                    } else {
                        None
                    }
                }
            },
            _ => None,
        }
    }

    /// Collect `ExportModule` from exports with declare statements.
    ///
    /// - `export var ...`
    /// - `export class ...`
    /// - `export function ...`
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
    fn collect_and_convert_export_decl(&mut self, export_decl: &ExportDecl) -> Option<Stmt> {
        debug!("export decl {:#?}", export_decl);
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

                    if self.runtime_module {
                        Some(Stmt::Decl(Decl::Var(Box::new(*var_decl.clone()))))
                    } else {
                        None
                    }
                } else {
                    unimplemented!();
                }
            }
            Decl::Fn(fn_decl @ FnDecl { ident, .. }) => {
                debug!("export decl fn: {:#?}", ident.sym);
                self.exports.push(ExportModule::named(ident.clone(), None));
                if self.runtime_module {
                    Some(Stmt::Decl(Decl::Fn(fn_decl.clone())))
                } else {
                    None
                }
            }
            Decl::Class(class_decl @ ClassDecl { ident, .. }) => {
                debug!("export decl class: {:#?}", ident.sym);
                self.exports.push(ExportModule::named(ident.clone(), None));
                if self.runtime_module {
                    Some(Stmt::Decl(Decl::Class(class_decl.clone())))
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}

impl VisitMut for EsModuleCollector {
    noop_visit_mut_type!();

    fn visit_mut_module(&mut self, module: &mut Module) {
        module.visit_mut_children_with(self);
    }

    fn visit_mut_module_items(&mut self, stmts: &mut Vec<ModuleItem>) {
        for stmt in stmts.iter_mut() {
            match stmt {
                ModuleItem::ModuleDecl(module_decl) => match module_decl {
                    ModuleDecl::Import(_) => {
                        module_decl.visit_mut_children_with(self);
                    }
                    ModuleDecl::ExportDefaultExpr(export_default_expr) => {
                        *stmt = self
                            .collect_and_convert_export_default_expr(export_default_expr)
                            .into();
                    }
                    ModuleDecl::ExportDefaultDecl(export_default_decl) => {
                        if let Some(converted_stmt) =
                            self.collect_and_convert_export_default_decl(export_default_decl)
                        {
                            *stmt = converted_stmt.into();
                        }
                    }
                    ModuleDecl::ExportDecl(export_decl) => {
                        if let Some(converted_stmt) =
                            self.collect_and_convert_export_decl(export_decl)
                        {
                            *stmt = converted_stmt.into();
                        }
                    }
                    _ => module_decl.visit_mut_children_with(self),
                },
                _ => {}
            }

            if self.runtime_module && stmt.is_module_decl() {
                stmt.take();
            }
        }

        stmts.retain(|stmt| {
            if let Some(module_decl) = stmt.as_module_decl() {
                !is_invalid_module_decl(module_decl)
            } else if matches!(stmt, ModuleItem::Stmt(Stmt::Empty(..))) {
                false
            } else {
                true
            }
        });
    }

    /// Collect `ExportImport` from import statements.
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
    fn visit_mut_import_decl(&mut self, import_decl: &mut ImportDecl) {
        debug!("import decl {:#?}", import_decl);

        // Collect when `runtime_module` is `true`.
        // If non-runtime, import statements will be striped by `take()`.
        if !self.runtime_module {
            return;
        }

        import_decl.specifiers.iter().for_each(|import_spec| {
            let src = import_decl.src.value.to_string();
            match import_spec {
                ImportSpecifier::Default(ImportDefaultSpecifier { local, .. }) => {
                    debug!("default import: {:#?}", local.sym);
                    self.imports
                        .push(ImportModule::default(local.clone(), None, src));
                }
                ImportSpecifier::Named(import_named_spec) => match import_named_spec {
                    ImportNamedSpecifier {
                        local,
                        imported: Some(imported),
                        ..
                    } => match imported {
                        ModuleExportName::Ident(imported_ident) => {
                            debug!("named import(alias): {:#?}", local.sym);
                            self.imports.push(ImportModule::named(
                                local.clone(),
                                Some(imported_ident.clone()),
                                src,
                            ));
                        }
                        ModuleExportName::Str(_) => unimplemented!(),
                    },
                    ImportNamedSpecifier {
                        local,
                        imported: None,
                        ..
                    } => {
                        debug!("named import: {:#?}", local.sym);
                        self.imports
                            .push(ImportModule::named(local.clone(), None, src));
                    }
                },
                ImportSpecifier::Namespace(ImportStarAsSpecifier { local, .. }) => {
                    debug!("namespace import: {:#?}", local.sym);
                    self.imports
                        .push(ImportModule::namespace(local.clone(), None, src));
                }
            }
        });
    }

    /// Collect `ExportModule` from named export statements.
    ///
    /// (Named exports)
    /// - `export { ... }`
    /// - `export { ident as ... }`
    /// - `export { default as ... }`
    ///
    /// (Re-exports)
    /// - `export * as ... from '...'`
    /// - `export { ... } from '...'`
    fn visit_mut_named_export(&mut self, named_export: &mut NamedExport) {
        debug!("named export {:#?}", named_export);
        match named_export {
            // Without source
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
            // With source (re-export)
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
                        None,
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
                            let ident = private_ident!("__re_export");
                            self.imports.push(ImportModule {
                                ident: ident.clone(),
                                imported: Some(orig_ident.clone()),
                                module_src: module_src.value.to_string(),
                                module_type: if is_default {
                                    ModuleType::DefaultAsNamed
                                } else {
                                    ModuleType::Named
                                },
                            });

                            match &exported {
                                Some(ModuleExportName::Ident(as_ident)) => self
                                    .exports
                                    .push(ExportModule::named(ident, as_ident.clone().into())),
                                Some(ModuleExportName::Str(_)) => unimplemented!(),
                                None => self
                                    .exports
                                    .push(ExportModule::named(ident, orig_ident.clone().into())),
                            }
                        }
                    });
                }
            }
        }
    }

    /// Collect `ExportModule` from export all statements.
    ///
    /// - `export * from ...`
    fn visit_mut_export_all(&mut self, export_all: &mut ExportAll) {
        debug!("export all {:#?}", export_all);
        let export_all_ident = private_ident!("__re_export_all");
        self.imports.push(ImportModule::namespace(
            export_all_ident.clone(),
            None,
            export_all.src.value.to_string(),
        ));
        self.exports
            .push(ExportModule::all(export_all_ident.clone(), None));
    }
}
