use swc_core::{
    atoms::Atom,
    common::{Span, DUMMY_SP},
    ecma::ast::*,
};

pub fn ident(sym: Atom) -> Ident {
    Ident::new(sym, DUMMY_SP)
}

pub fn ident_expr(sym: Atom) -> Expr {
    Expr::Ident(ident(sym))
}

pub fn str_lit_expr(value: String) -> Expr {
    Expr::Lit(Lit::Str(Str {
        span: DUMMY_SP,
        value: value.to_owned().into(),
        raw: None,
    }))
}

pub fn fn_arg(expr: Expr) -> ExprOrSpread {
    ExprOrSpread {
        expr: Box::new(expr),
        spread: None,
    }
}

pub fn obj_member_expr(obj: Expr, prop: Ident) -> Expr {
    Expr::Member(MemberExpr {
        span: DUMMY_SP,
        obj: Box::new(obj),
        prop: MemberProp::Ident(prop),
    })
}

pub fn call_expr(callee: Expr, args: Vec<ExprOrSpread>) -> Expr {
    Expr::Call(CallExpr {
        span: DUMMY_SP,
        callee: Callee::Expr(Box::new(callee)),
        args: args,
        type_args: None,
    })
}

pub fn decl_var_and_assign_stmt(name: Ident, span: Span, init: Expr) -> Stmt {
    Stmt::Decl(Decl::Var(Box::new(VarDecl {
        span: DUMMY_SP,
        kind: VarDeclKind::Var,
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
