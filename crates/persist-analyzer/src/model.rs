//! Builds a lightweight model of `soroban_sdk::storage` API usage out of a
//! parsed source file, by pattern-matching method-call chains shaped like
//! `<env>.storage().<kind>().<op>(...)`.
//!
//! This is deliberately syntactic rather than type-checked: we don't run a
//! full rustc/dylint pipeline, we look for the exact call shape the
//! `soroban-sdk` storage API always takes. See the crate README's "How it
//! works" section for the tradeoffs this implies.

use proc_macro2::Span;
use quote::quote;
use syn::visit::{self, Visit};
use syn::{Expr, ExprMethodCall, ImplItemFn, ItemFn, ItemImpl};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StorageKind {
    Persistent,
    Instance,
    Temporary,
}

impl StorageKind {
    fn from_method_name(name: &str) -> Option<Self> {
        match name {
            "persistent" => Some(StorageKind::Persistent),
            "instance" => Some(StorageKind::Instance),
            "temporary" => Some(StorageKind::Temporary),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            StorageKind::Persistent => "persistent",
            StorageKind::Instance => "instance",
            StorageKind::Temporary => "temporary",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpKind {
    Set,
    Get,
    Has,
    Remove,
    ExtendTtl,
}

impl OpKind {
    fn from_method_name(name: &str) -> Option<Self> {
        match name {
            "set" => Some(OpKind::Set),
            "get" => Some(OpKind::Get),
            "has" => Some(OpKind::Has),
            "remove" => Some(OpKind::Remove),
            "extend_ttl" => Some(OpKind::ExtendTtl),
            _ => None,
        }
    }
}

/// One recognized `storage().<kind>().<op>(...)` call site.
#[derive(Debug, Clone)]
pub struct StorageOp {
    pub kind: StorageKind,
    pub op: OpKind,
    /// Enclosing `impl Type { ... }` name, if any.
    pub impl_name: Option<String>,
    /// Enclosing function/method name.
    pub fn_name: String,
    /// Raw arguments passed to the op call, in source order.
    pub args: Vec<Expr>,
    /// Best-effort stringified form of the first argument (the storage
    /// key), used by heuristics that key off of naming.
    pub key_repr: Option<String>,
    pub line: usize,
    pub column: usize,
}

/// A direct `<get-call>.unwrap()` / `<get-call>.expect(..)` chain, i.e. a
/// storage read whose "entry missing / archived" case is not handled.
#[derive(Debug, Clone)]
pub struct UnwrappedGet {
    pub kind: StorageKind,
    pub impl_name: Option<String>,
    pub fn_name: String,
    pub via: &'static str, // "unwrap" or "expect"
    pub line: usize,
    pub column: usize,
}

#[derive(Debug, Clone, Default)]
pub struct FileModel {
    pub ops: Vec<StorageOp>,
    pub unwrapped_gets: Vec<UnwrappedGet>,
}

/// Returns `Some((kind, op_ident, args))` if `call` is shaped like
/// `<recv>.storage().<kind>().<op>(args)`.
fn match_storage_chain(call: &ExprMethodCall) -> Option<(StorageKind, OpKind, Vec<Expr>)> {
    let op = OpKind::from_method_name(&call.method.to_string())?;
    let kind_call = match &*call.receiver {
        Expr::MethodCall(inner) => inner,
        _ => return None,
    };
    let kind = StorageKind::from_method_name(&kind_call.method.to_string())?;
    let storage_call = match &*kind_call.receiver {
        Expr::MethodCall(inner) => inner,
        _ => return None,
    };
    if storage_call.method != "storage" {
        return None;
    }
    let args: Vec<Expr> = call.args.iter().cloned().collect();
    Some((kind, op, args))
}

/// Returns `Some(kind)` if `call` is a bare `<recv>.storage().<kind>().get(..)`
/// call (used to recognize `.get(..).unwrap()` / `.get(..).expect(..)`).
fn match_get_receiver(expr: &Expr) -> Option<StorageKind> {
    if let Expr::MethodCall(call) = expr {
        if let Some((kind, OpKind::Get, _)) = match_storage_chain(call) {
            return Some(kind);
        }
    }
    None
}

fn line_col(span: Span) -> (usize, usize) {
    let start = span.start();
    (start.line, start.column + 1)
}

struct Collector {
    model: FileModel,
    impl_stack: Vec<String>,
    fn_stack: Vec<String>,
}

impl Collector {
    fn new() -> Self {
        Collector {
            model: FileModel::default(),
            impl_stack: Vec::new(),
            fn_stack: Vec::new(),
        }
    }

    fn current_impl(&self) -> Option<String> {
        self.impl_stack.last().cloned()
    }

    fn current_fn(&self) -> String {
        self.fn_stack
            .last()
            .cloned()
            .unwrap_or_else(|| "<module>".to_string())
    }
}

impl<'ast> Visit<'ast> for Collector {
    fn visit_item_impl(&mut self, node: &'ast ItemImpl) {
        let self_ty = &*node.self_ty;
        let name = quote!(#self_ty).to_string().replace(' ', "");
        self.impl_stack.push(name);
        visit::visit_item_impl(self, node);
        self.impl_stack.pop();
    }

    fn visit_item_fn(&mut self, node: &'ast ItemFn) {
        self.fn_stack.push(node.sig.ident.to_string());
        visit::visit_item_fn(self, node);
        self.fn_stack.pop();
    }

    fn visit_impl_item_fn(&mut self, node: &'ast ImplItemFn) {
        self.fn_stack.push(node.sig.ident.to_string());
        visit::visit_impl_item_fn(self, node);
        self.fn_stack.pop();
    }

    fn visit_expr_method_call(&mut self, node: &'ast ExprMethodCall) {
        let method = node.method.to_string();

        if (method == "unwrap" || method == "expect") && node.args.len() <= 1 {
            if let Some(kind) = match_get_receiver(&node.receiver) {
                let (line, column) = line_col(node.method.span());
                self.model.unwrapped_gets.push(UnwrappedGet {
                    kind,
                    impl_name: self.current_impl(),
                    fn_name: self.current_fn(),
                    via: if method == "unwrap" {
                        "unwrap"
                    } else {
                        "expect"
                    },
                    line,
                    column,
                });
            }
        }

        if let Some((kind, op, args)) = match_storage_chain(node) {
            let key_repr = args.first().map(|e| quote!(#e).to_string());
            let (line, column) = line_col(node.method.span());
            self.model.ops.push(StorageOp {
                kind,
                op,
                impl_name: self.current_impl(),
                fn_name: self.current_fn(),
                args,
                key_repr,
                line,
                column,
            });
        }

        // Keep descending so nested calls (e.g. inside closures/args) are
        // still visited; recognized chains are only ever matched once,
        // at their outermost `.op(..)` call, since `persistent()`/`get()`
        // alone don't satisfy `match_storage_chain` on their own.
        visit::visit_expr_method_call(self, node);
    }
}

pub fn build_model(file: &syn::File) -> FileModel {
    let mut collector = Collector::new();
    collector.visit_file(file);
    collector.model
}
