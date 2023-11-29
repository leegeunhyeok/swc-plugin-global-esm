use crate::utils::decl_var_and_assign_stmt;
use swc_core::{
    common::DUMMY_SP,
    ecma::{
        ast::*,
        utils::private_ident,
        visit::{VisitMut, VisitMutWith},
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

pub struct ModuleCollector {
    pub imports: Vec<ImportModule>,
    pub exports: Vec<ExportModule>,
    runtime_module: bool,
}

impl ModuleCollector {
    pub fn default(runtime_module: bool) -> Self {
        ModuleCollector {
            runtime_module,
            imports: Vec::new(),
            exports: Vec::new(),
        }
    }

    /// Returns private `Ident` and declare ident as variable statement.
    ///
    /// eg. `const __export_default = expr`
    fn get_export_decl_stmt_with_private_ident(&mut self, expr: Expr) -> (Ident, Stmt) {
        let export_ident = private_ident!("__export_default");
        let stmt = decl_var_and_assign_stmt(export_ident.clone(), DUMMY_SP, expr);
        (export_ident, stmt)
    }

    /// Returns an default export statement.
    ///
    /// eg. `export default ident`
    fn get_default_export_stmt(&mut self, ident: Ident) -> ModuleDecl {
        ExportDefaultExpr {
            span: DUMMY_SP,
            expr: ident.into(),
        }
        .into()
    }

    fn collect_default_export_decl_and_convert_to_stmt(
        &mut self,
        export_default_decl: &ExportDefaultDecl,
    ) -> Option<(Ident, Stmt)> {
        match &export_default_decl.decl {
            DefaultDecl::Fn(FnExpr {
                ident: Some(fn_ident),
                function,
                ..
            }) => {
                debug!("default export decl fn: {:#?}", fn_ident.sym);
                self.exports.push(ExportModule::default(fn_ident.clone()));
                Some((
                    fn_ident.clone(),
                    FnDecl {
                        ident: fn_ident.clone(),
                        function: function.clone(),
                        declare: false,
                    }
                    .into(),
                ))
            }
            DefaultDecl::Fn(fn_expr) => {
                debug!("default export decl fn: <anonymous>");
                let (ident, stmt) =
                    self.get_export_decl_stmt_with_private_ident(fn_expr.clone().into());
                self.exports.push(ExportModule::default(ident.clone()));
                Some((ident, stmt))
            }
            DefaultDecl::Class(ClassExpr {
                ident: Some(class_ident),
                class,
                ..
            }) => {
                debug!("default export decl class: {:#?}", class_ident.sym);
                self.exports
                    .push(ExportModule::default(class_ident.clone()));
                Some((
                    class_ident.clone(),
                    ClassDecl {
                        ident: class_ident.clone(),
                        class: class.clone(),
                        declare: false,
                    }
                    .into(),
                ))
            }
            DefaultDecl::Class(class_expr) => {
                debug!("default export decl class: <anonymous>");
                let (ident, stmt) =
                    self.get_export_decl_stmt_with_private_ident(class_expr.clone().into());
                self.exports.push(ExportModule::default(ident.clone()));
                Some((ident, stmt))
            }
            _ => None,
        }
    }

    fn collect_default_export_expr_and_convert_to_stmt(
        &mut self,
        export_default_expr: &ExportDefaultExpr,
    ) -> (Ident, Stmt) {
        let (ident, stmt) =
            self.get_export_decl_stmt_with_private_ident(*export_default_expr.expr.clone());
        self.exports.push(ExportModule::default(ident.clone()));
        (ident, stmt)
    }

    fn collect_named_export_specifiers(&mut self, specifiers: &Vec<ExportSpecifier>) {
        specifiers
            .to_owned()
            .into_iter()
            .for_each(|export_spec| match export_spec {
                ExportSpecifier::Named(ExportNamedSpecifier {
                    orig: ModuleExportName::Ident(orig_ident),
                    exported,
                    is_type_only: false,
                    ..
                }) => match &exported {
                    Some(ModuleExportName::Ident(as_ident)) => self.exports.push(
                        ExportModule::named(orig_ident.clone(), Some(as_ident.clone())),
                    ),
                    _ => self
                        .exports
                        .push(ExportModule::named(orig_ident.clone(), None)),
                },
                _ => {}
            });
    }

    fn collect_named_re_export_specifiers(&mut self, specifiers: &Vec<ExportSpecifier>, src: &Str) {
        specifiers
            .to_owned()
            .into_iter()
            .for_each(|import_spec| match import_spec {
                ExportSpecifier::Named(ExportNamedSpecifier { orig, exported, .. }) => {
                    if let ModuleExportName::Ident(orig_ident) = &orig {
                        let is_default = orig_ident.sym == "default";
                        let target_ident = if is_default {
                            private_ident!("__default")
                        } else {
                            private_ident!(orig_ident.span, orig_ident.sym.clone())
                        };
                        self.imports.push(ImportModule {
                            ident: target_ident.clone(),
                            module_src: src.value.to_string(),
                            module_type: if is_default {
                                ModuleType::DefaultAsNamed
                            } else {
                                ModuleType::Named
                            },
                        });
                        match &exported {
                            Some(ModuleExportName::Ident(as_ident)) => self
                                .exports
                                .push(ExportModule::named(target_ident, Some(as_ident.clone()))),
                            _ => self
                                .exports
                                .push(ExportModule::named(target_ident, Some(orig_ident.clone()))),
                        }
                    }
                }
                _ => {}
            });
    }
}

impl VisitMut for ModuleCollector {
    fn visit_mut_module(&mut self, module: &mut Module) {
        let mut module_body = Vec::with_capacity(module.body.len());
        for module_item in module.body.drain(..) {
            match module_item {
                ModuleItem::Stmt(stmt) => module_body.push(stmt.into()),
                ModuleItem::ModuleDecl(mut module_decl) => match &module_decl {
                    // Imports
                    ModuleDecl::Import(_) => {
                        if self.runtime_module {
                            module_decl.visit_mut_children_with(self);
                        } else {
                            module_body.push(module_decl.into());
                        }
                    }
                    // Exports
                    // `export var ...`
                    // `export class ...`
                    // `export function ...`
                    ModuleDecl::ExportDecl(export_decl) => {
                        if self.runtime_module {
                            module_body.push(Stmt::Decl(export_decl.decl.clone()).into());
                        } else {
                            module_body.push(module_decl.clone().into());
                        }
                        module_decl.visit_mut_children_with(self);
                    }
                    // `export default function ...`
                    // `export default class ...`
                    ModuleDecl::ExportDefaultDecl(export_default_decl) => {
                        if let Some((ident, export_stmt)) = self
                            .collect_default_export_decl_and_convert_to_stmt(export_default_decl)
                        {
                            module_body.push(export_stmt.into());
                            if !self.runtime_module {
                                module_body.push(self.get_default_export_stmt(ident).into());
                            }
                        } else {
                            module_body.push(module_decl.into());
                        }
                    }
                    // `export default Identifier`
                    ModuleDecl::ExportDefaultExpr(export_default_expr) => {
                        let (ident, stmt) = self
                            .collect_default_export_expr_and_convert_to_stmt(export_default_expr);
                        module_body.push(stmt.into());
                        if !self.runtime_module {
                            module_body.push(self.get_default_export_stmt(ident).into());
                        }
                    }
                    // Named export
                    // `export { ... }`
                    // `export { ident as ... }`
                    // `export { default as ... }`
                    //
                    // Export all
                    // `export * from ...`
                    ModuleDecl::ExportNamed(NamedExport {
                        type_only: false, ..
                    })
                    | ModuleDecl::ExportAll(ExportAll {
                        type_only: false, ..
                    }) => {
                        module_decl.visit_mut_children_with(self);
                        if !self.runtime_module {
                            module_body.push(module_decl.into());
                        }
                    }
                    _ => {
                        if !self.runtime_module {
                            module_body.push(module_decl.into());
                        }
                    }
                },
            };
        }
        module.body = module_body;
    }

    fn visit_mut_import_decl(&mut self, import_decl: &mut ImportDecl) {
        import_decl
            .specifiers
            .to_owned()
            .into_iter()
            .for_each(|import_spec| {
                let module_src = import_decl.src.value.to_string();
                match import_spec {
                    ImportSpecifier::Default(ImportDefaultSpecifier { local, .. }) => {
                        debug!("default import: {:#?}", local.sym);
                        self.imports.push(ImportModule::default(local, module_src));
                    }
                    ImportSpecifier::Named(ImportNamedSpecifier { local, .. }) => {
                        debug!("named import: {:#?}", local.sym);
                        self.imports.push(ImportModule::named(local, module_src));
                    }
                    ImportSpecifier::Namespace(ImportStarAsSpecifier { local, .. }) => {
                        debug!("namespace import: {:#?}", local.sym);
                        self.imports
                            .push(ImportModule::namespace(local, module_src));
                    }
                }
            });
    }

    fn visit_mut_export_decl(&mut self, export_decl: &mut ExportDecl) {
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
    }

    fn visit_mut_named_export(&mut self, named_export: &mut NamedExport) {
        debug!("named export {:#?}", named_export);
        match named_export {
            // without source
            // `export { ... };`
            NamedExport {
                src: None,
                specifiers,
                ..
            } => self.collect_named_export_specifiers(specifiers),
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
                        Some(module_ident.clone()),
                    ));
                } else {
                    // Case 2
                    self.collect_named_re_export_specifiers(specifiers, module_src);
                }
            }
        }
    }

    fn visit_mut_export_all(&mut self, export_all: &mut ExportAll) {
        debug!("export all {:#?}", export_all);
        let export_all_ident = private_ident!("__re_export_all");
        self.imports.push(ImportModule {
            ident: export_all_ident.clone(),
            module_src: export_all.src.value.to_string(),
            module_type: ModuleType::NamespaceOrAll,
        });
        self.exports
            .push(ExportModule::all(export_all_ident.clone(), None));
    }
}
