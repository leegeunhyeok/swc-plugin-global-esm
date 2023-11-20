use swc_core::{
    atoms::Atom,
    common::{Span, DUMMY_SP},
    ecma::ast::*,
};

/// Returns an `Ident`.
pub fn ident(sym: Atom) -> Ident {
    Ident::new(sym, DUMMY_SP)
}

/// Returns an expression of `Ident`.
pub fn ident_expr(sym: Atom) -> Expr {
    Expr::Ident(ident(sym))
}

/// Returns a string literal expression.
///
/// eg. `"value"`
pub fn str_lit_expr(value: String) -> Expr {
    Expr::Lit(Lit::Str(Str {
        span: DUMMY_SP,
        value: value.to_owned().into(),
        raw: None,
    }))
}

/// Returns a function arguments expression.
///
/// eg. `expr` in `fn(expr)`
pub fn fn_arg(expr: Expr) -> ExprOrSpread {
    ExprOrSpread {
        expr: Box::new(expr),
        spread: None,
    }
}

/// Returns an object member expression.
///
/// eg. `obj.prop`
pub fn obj_member_expr(obj: Expr, prop: Ident) -> Expr {
    Expr::Member(MemberExpr {
        span: DUMMY_SP,
        obj: Box::new(obj),
        prop: MemberProp::Ident(prop),
    })
}

/// Returns a function call expression.
///
/// eg. `callee(arg1, arg2, ...)`
pub fn call_expr(callee: Expr, args: Vec<ExprOrSpread>) -> Expr {
    Expr::Call(CallExpr {
        span: DUMMY_SP,
        callee: Callee::Expr(Box::new(callee)),
        args: args,
        type_args: None,
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
            name: Pat::Ident(BindingIdent {
                id: name,
                type_ann: None,
            }),
            init: Some(Box::new(init)),
            definite: false,
        }],
    })))
}
