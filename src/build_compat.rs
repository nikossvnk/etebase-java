//! Remove flapigen's unused numeric Optional caches, which require Android API 24.
//!
//! Our public bindings use boxed nullable values instead. Keep this transformation
//! platform-independent so the JVM missing-class smoke exercises the Android fix.

use proc_macro2::{Ident, TokenStream, TokenTree};
use quote::ToTokens;
use std::{collections::BTreeSet, fs, path::Path, process::Command};
use syn::{visit::Visit, Expr, Item, Stmt, Visibility};

const HELPERS: [&str; 3] = [
    "to_java_util_optional_double",
    "to_java_util_optional_long",
    "to_java_util_optional_int",
];
const CACHES: [[&str; 3]; 3] = [
    [
        "JAVA_UTIL_OPTIONAL_DOUBLE",
        "JAVA_UTIL_OPTIONAL_DOUBLE_OF",
        "JAVA_UTIL_OPTIONAL_DOUBLE_EMPTY",
    ],
    [
        "JAVA_UTIL_OPTIONAL_LONG",
        "JAVA_UTIL_OPTIONAL_LONG_OF",
        "JAVA_UTIL_OPTIONAL_LONG_EMPTY",
    ],
    [
        "JAVA_UTIL_OPTIONAL_INT",
        "JAVA_UTIL_OPTIONAL_INT_OF",
        "JAVA_UTIL_OPTIONAL_INT_EMPTY",
    ],
];

#[derive(Default)]
struct OptionalReferences {
    caches: BTreeSet<&'static str>,
    helpers: BTreeSet<&'static str>,
}

impl OptionalReferences {
    fn record_ident(&mut self, ident: &Ident) {
        if let Some(name) = CACHES.iter().flatten().find(|name| ident == **name) {
            self.caches.insert(*name);
        } else {
            assert!(
                !ident.to_string().starts_with("JAVA_UTIL_OPTIONAL_"),
                "Unexpected flapigen Optional cache identifier: {}",
                ident,
            );
        }
        if let Some(name) = HELPERS.iter().find(|name| ident == **name) {
            self.helpers.insert(*name);
        }
    }

    fn record_tokens(&mut self, tokens: TokenStream) {
        for token in tokens {
            match token {
                TokenTree::Ident(ident) => self.record_ident(&ident),
                TokenTree::Group(group) => self.record_tokens(group.stream()),
                _ => {}
            }
        }
    }
}

impl<'ast> Visit<'ast> for OptionalReferences {
    fn visit_ident(&mut self, ident: &'ast Ident) {
        self.record_ident(ident);
    }

    fn visit_token_stream(&mut self, tokens: &'ast TokenStream) {
        // syn deliberately treats macro bodies as opaque; cache/helper uses in
        // those bodies must also participate in the fail-closed checks.
        self.record_tokens(tokens.clone());
    }
}

fn prune_lifecycle(function: &mut syn::ItemFn, loading: bool) {
    let mut removed = [false; 3];
    let name = &function.sig.ident;
    function.block.stmts.retain(|statement| {
        let mut references = OptionalReferences::default();
        references.visit_stmt(statement);
        if references.caches.is_empty() {
            return true;
        }
        let block = match statement {
            Stmt::Expr(Expr::Unsafe(block), None) => block,
            _ => panic!("Optional cache is not a standalone unsafe block in {}", name),
        };
        assert!(
            block.attrs.is_empty() && block.block.stmts.len() == if loading { 12 } else { 2 },
            "Unexpected Optional unsafe block shape in {}",
            name,
        );
        let index = CACHES
            .iter()
            .position(|names| references.caches.contains(names[0]))
            .expect("Optional lifecycle block has no class cache");
        let expected: BTreeSet<_> = if loading {
            CACHES[index].iter().copied().collect()
        } else {
            [CACHES[index][0]].iter().copied().collect()
        };
        assert!(
            references.caches == expected && references.helpers.is_empty(),
            "Unexpected Optional lifecycle block in {}",
            name,
        );
        assert!(
            !removed[index],
            "Duplicate Optional lifecycle block in {}",
            name,
        );
        removed[index] = true;
        false
    });
    assert!(
        removed.iter().all(|removed| *removed),
        "Expected three Optional lifecycle blocks in {}",
        name,
    );
}

pub fn prune_unused_numeric_optionals(path: &Path) {
    let source = fs::read_to_string(path).expect("read generated JNI bindings");
    let mut file = syn::parse_file(&source).expect("parse generated JNI bindings");
    let mut helpers = [false; 3];
    let mut caches = [[false; 3]; 3];
    let mut lifecycle = [false; 2];
    file.items.retain_mut(|item| {
        match item {
            Item::Fn(function) => {
                if let Some(index) = HELPERS.iter().position(|name| function.sig.ident == *name) {
                    assert!(
                        matches!(function.vis, Visibility::Inherited) && !helpers[index],
                        "Expected one private Optional helper: {}",
                        function.sig.ident,
                    );
                    helpers[index] = true;
                    return false;
                }
                for (index, name) in ["JNI_OnLoad", "JNI_OnUnload"].iter().enumerate() {
                    if function.sig.ident == *name {
                        assert!(!lifecycle[index], "Duplicate {} function", name);
                        lifecycle[index] = true;
                        prune_lifecycle(function, index == 0);
                    }
                }
            }
            Item::Static(cache) => {
                for (class_index, names) in CACHES.iter().enumerate() {
                    if let Some(index) = names.iter().position(|name| cache.ident == *name) {
                        assert!(
                            matches!(cache.vis, Visibility::Inherited)
                                && !caches[class_index][index],
                            "Expected one private Optional cache: {}",
                            cache.ident,
                        );
                        caches[class_index][index] = true;
                        return false;
                    }
                }
            }
            _ => {}
        }
        true
    });
    assert!(
        helpers.iter().all(|found| *found)
            && caches.iter().flatten().all(|found| *found)
            && lifecycle.iter().all(|found| *found),
        "Expected exactly three Optional helpers, nine caches and both JNI lifecycle functions",
    );

    // A new public conversion or unexpected generator layout must fail the
    // build, rather than silently removing code that has become reachable.
    let mut remaining = OptionalReferences::default();
    remaining.visit_file(&file);
    assert!(
        remaining.caches.is_empty() && remaining.helpers.is_empty(),
        "Numeric Optional references remain after pruning; review the generated JNI API",
    );

    fs::write(path, file.into_token_stream().to_string()).expect("write pruned JNI bindings");
    let status = Command::new("rustfmt")
        .arg("--edition")
        .arg("2018")
        .arg(path)
        .status()
        .expect("run rustfmt on pruned JNI bindings");
    assert!(status.success(), "rustfmt failed for pruned JNI bindings");
}
