use crate::utils::decl_var_and_assign_stmt;
use swc_core::{
    common::{Span, DUMMY_SP},
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
    // import: namespace
    // export: all
    NamespaceOrAll,
}

#[derive(Debug)]
pub struct ImportModule {
    pub span: Span,
    pub ident: Ident,
    pub module_src: String,
    pub module_type: ModuleType,
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

    fn get_export_decl_stmt_with_private_ident(&mut self, expr: Expr) -> (Ident, Stmt) {
        let export_ident: Ident = private_ident!("__export_default");
        let stmt = decl_var_and_assign_stmt(export_ident.clone(), DUMMY_SP, expr);
        (export_ident, stmt)
    }

    fn get_default_export_stmt(&mut self, ident: Ident) -> ModuleDecl {
        ModuleDecl::ExportDefaultExpr(ExportDefaultExpr {
            span: DUMMY_SP,
            expr: Box::new(Expr::Ident(ident)),
        })
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
                    fn_ident.to_owned(),
                    Stmt::Decl(Decl::Fn(FnDecl {
                        ident: fn_ident.to_owned(),
                        function: function.to_owned(),
                        declare: false,
                    })),
                ))
            }
            DefaultDecl::Fn(fn_expr) => {
                debug!("default export decl fn: <anonymous>");
                let (ident, stmt) =
                    self.get_export_decl_stmt_with_private_ident(Expr::Fn(fn_expr.to_owned()));
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
                    class_ident.to_owned(),
                    Stmt::Decl(Decl::Class(ClassDecl {
                        ident: class_ident.to_owned(),
                        class: class.to_owned(),
                        declare: false,
                    })),
                ))
            }
            DefaultDecl::Class(class_expr) => {
                debug!("default export decl class: <anonymous>");
                let (ident, stmt) = self
                    .get_export_decl_stmt_with_private_ident(Expr::Class(class_expr.to_owned()));
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
            self.get_export_decl_stmt_with_private_ident(*export_default_expr.expr.to_owned());
        self.exports.push(ExportModule::default(ident.clone()));
        (ident, stmt)
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
                            module_decl.visit_mut_with(self);
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
                        module_decl.visit_mut_with(self);
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
                    // `export { ... }`
                    ModuleDecl::ExportNamed(NamedExport {
                        type_only: false, ..
                    }) => {
                        module_decl.visit_mut_with(self);
                        if !self.runtime_module {
                            module_body.push(module_decl.into());
                        }
                    }
                    // `export * from ...`
                    ModuleDecl::ExportAll(ExportAll {
                        type_only: false, ..
                    }) => {
                        module_decl.visit_mut_with(self);
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
            .for_each(|import_spec| match import_spec {
                ImportSpecifier::Default(ImportDefaultSpecifier { span, local }) => {
                    debug!("default import: {:#?}", local.sym);
                    self.imports.push(ImportModule {
                        span,
                        ident: local,
                        module_src: import_decl.src.value.to_string(),
                        module_type: ModuleType::Default,
                    });
                }
                ImportSpecifier::Named(ImportNamedSpecifier { span, local, .. }) => {
                    debug!("named import: {:#?}", local.sym);
                    self.imports.push(ImportModule {
                        span,
                        ident: local,
                        module_src: import_decl.src.value.to_string(),
                        module_type: ModuleType::Named,
                    });
                }
                ImportSpecifier::Namespace(ImportStarAsSpecifier { span, local }) => {
                    debug!("namespace import: {:#?}", local.sym);
                    self.imports.push(ImportModule {
                        span,
                        ident: local,
                        module_src: import_decl.src.value.to_string(),
                        module_type: ModuleType::NamespaceOrAll,
                    });
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
            _ => (),
        }
    }

    fn visit_mut_named_export(&mut self, named_export: &mut NamedExport) {
        match named_export {
            NamedExport { src: None, .. } => named_export.visit_mut_children_with(self),
            NamedExport {
                src: Some(module_src),
                ..
            } => {
                if let Some(ExportSpecifier::Namespace(ExportNamespaceSpecifier {
                    span,
                    name: ModuleExportName::Ident(module_ident),
                })) = named_export.specifiers.get(0)
                {
                    debug!("namespace export: {:#?}", module_ident.sym);
                    let export_ident: Ident = private_ident!("__export_named");
                    self.imports.push(ImportModule {
                        span: *span,
                        ident: export_ident.clone(),
                        module_src: module_src.value.to_string(),
                        module_type: ModuleType::NamespaceOrAll,
                    });
                    self.exports.push(ExportModule::named(
                        export_ident,
                        Some(module_ident.to_owned()),
                    ));
                } else {
                    named_export
                        .specifiers
                        .to_owned()
                        .into_iter()
                        .for_each(|import_spec| match import_spec {
                            ExportSpecifier::Named(ExportNamedSpecifier { span, orig, .. }) => {
                                if let ModuleExportName::Ident(orig_ident) = &orig {
                                    debug!("named export: {:#?}", orig_ident.sym);
                                    self.imports.push(ImportModule {
                                        span,
                                        ident: orig_ident.clone(),
                                        module_src: module_src.value.to_string(),
                                        module_type: ModuleType::Named,
                                    });
                                }
                            }
                            _ => (),
                        });
                    named_export.visit_mut_children_with(self);
                }
            }
        }
    }

    fn visit_mut_export_named_specifier(&mut self, named_spec: &mut ExportNamedSpecifier) {
        if let ModuleExportName::Ident(orig_ident) = &named_spec.orig {
            debug!("named export: {:#?}", orig_ident.sym);
            match &named_spec.exported {
                Some(ModuleExportName::Ident(as_ident)) => self.exports.push(ExportModule::named(
                    orig_ident.clone(),
                    Some(as_ident.clone()),
                )),
                _ => self
                    .exports
                    .push(ExportModule::named(orig_ident.clone(), None)),
            }
        }
    }

    fn visit_mut_export_all(&mut self, export_all: &mut ExportAll) {
        let export_all_ident: Ident = private_ident!("__export_all");
        self.imports.push(ImportModule {
            span: DUMMY_SP,
            ident: export_all_ident.clone(),
            module_src: export_all.src.value.to_string(),
            module_type: ModuleType::NamespaceOrAll,
        });
        self.exports.push(ExportModule {
            ident: export_all_ident.clone(),
            as_ident: None,
            module_type: ModuleType::NamespaceOrAll,
        });
    }
}
