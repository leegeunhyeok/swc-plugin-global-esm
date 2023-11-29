use swc_core::{
    common::{Span, DUMMY_SP},
    ecma::ast::*,
};

/// Returns an object member expression.
///
/// eg. `obj.prop`
pub fn obj_member_expr(obj: Expr, prop: Ident) -> Expr {
    Expr::Member(MemberExpr {
        span: DUMMY_SP,
        obj: obj.into(),
        prop: prop.into(),
    })
}

/// Returns an assign expression with declare variable statement.
///
/// eg. `const name = expr`
pub fn decl_var_and_assign_stmt(name: Ident, span: Span, init: Expr) -> Stmt {
    Stmt::Decl(Decl::Var(Box::new(VarDecl {
        span: DUMMY_SP,
        kind: VarDeclKind::Const,
        declare: false,
        decls: vec![VarDeclarator {
            span,
            name: name.into(),
            init: Some(init.into()),
            definite: false,
        }],
    })))
}
