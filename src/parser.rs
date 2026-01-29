use std::path::Path;
use std::rc::Rc;
use swc_common::SourceMap;
use swc_ecma_ast::{EsVersion, *};
use swc_ecma_parser::{lexer::Lexer, EsSyntax, Parser, StringInput, Syntax, TsSyntax};
use swc_ecma_visit::{noop_visit_type, Visit, VisitWith};

/// Options for import extraction
#[derive(Default, Clone)]
pub struct ParserOptions {
    /// If true, type-only imports (import type { Foo }) are excluded
    pub ignore_type_imports: bool,
}

pub fn get_imports_from_file(path: &Path, options: &ParserOptions) -> Vec<String> {
    let module = match parse_file_to_ast(path) {
        Some(m) => m,
        None => return vec![],
    };

    let mut collector = ImportCollector {
        imports: vec![],
        ignore_type_imports: options.ignore_type_imports,
    };
    module.visit_with(&mut collector);
    collector.imports
}

fn get_syntax_for_file(path: &Path) -> Syntax {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    match ext.as_str() {
        // TypeScript files
        "ts" => Syntax::Typescript(TsSyntax {
            tsx: false,
            decorators: true,
            dts: false,
            no_early_errors: false,
            ..Default::default()
        }),
        "tsx" => Syntax::Typescript(TsSyntax {
            tsx: true,
            decorators: true,
            dts: false,
            no_early_errors: false,
            ..Default::default()
        }),
        "dts" => Syntax::Typescript(TsSyntax {
            tsx: false,
            decorators: true,
            dts: true,
            no_early_errors: false,
            ..Default::default()
        }),
        // JavaScript files - use ES syntax
        "js" | "jsx" | "mjs" => Syntax::Es(EsSyntax {
            jsx: ext == "jsx",
            decorators: true,
            ..Default::default()
        }),
        // CommonJS - use ES syntax (we handle require() in the visitor)
        "cjs" => Syntax::Es(EsSyntax {
            jsx: false,
            decorators: false,
            ..Default::default()
        }),
        // Default to TypeScript for unknown extensions (most permissive)
        _ => Syntax::Typescript(TsSyntax {
            tsx: true,
            decorators: true,
            dts: false,
            no_early_errors: false,
            ..Default::default()
        }),
    }
}

fn parse_file_to_ast(path: &Path) -> Option<Module> {
    let cm = Rc::new(SourceMap::default());
    let fm = cm.load_file(path).ok()?;

    let syntax = get_syntax_for_file(path);
    let lexer = Lexer::new(syntax, EsVersion::Es2022, StringInput::from(&*fm), None);
    let mut parser = Parser::new_from(lexer);

    match parser.parse_module() {
        Ok(module) => Some(module),
        Err(err) => {
            eprintln!("Failed to parse {}: {:?}", path.display(), err);
            None
        }
    }
}

struct ImportCollector {
    imports: Vec<String>,
    ignore_type_imports: bool,
}

impl ImportCollector {
    /// Check if an import declaration should be included based on type-only status
    fn should_include_import(&self, import_decl: &ImportDecl) -> bool {
        if !self.ignore_type_imports {
            return true;
        }

        // Skip if the entire import is type-only: `import type { Foo } from './foo'`
        if import_decl.type_only {
            return false;
        }

        // For mixed imports like `import { type Foo, Bar } from './foo'`
        // Check if ALL specifiers are type-only
        let all_type_only = import_decl.specifiers.iter().all(|spec| {
            match spec {
                ImportSpecifier::Named(named) => named.is_type_only,
                // Default and namespace imports are value imports
                ImportSpecifier::Default(_) | ImportSpecifier::Namespace(_) => false,
            }
        });

        // If all specifiers are type-only, skip this import
        !all_type_only
    }
}

impl Visit for ImportCollector {
    noop_visit_type!();

    fn visit_module_item(&mut self, item: &ModuleItem) {
        match item {
            // ES Module imports: import { foo } from './foo'
            ModuleItem::ModuleDecl(ModuleDecl::Import(import_decl)) => {
                if self.should_include_import(import_decl) {
                    self.imports.push(import_decl.src.value.to_string());
                }
            }
            // Re-exports: export * from './foo'
            ModuleItem::ModuleDecl(ModuleDecl::ExportAll(export_all)) => {
                // export type * from './foo' is type-only
                if !(self.ignore_type_imports && export_all.type_only) {
                    self.imports.push(export_all.src.value.to_string());
                }
            }
            // Named re-exports: export { foo } from './foo'
            ModuleItem::ModuleDecl(ModuleDecl::ExportNamed(named_export)) => {
                if let Some(src) = &named_export.src {
                    // Check if this is a type-only export
                    if !(self.ignore_type_imports && named_export.type_only) {
                        self.imports.push(src.value.to_string());
                    }
                }
            }
            _ => {}
        }

        item.visit_children_with(self);
    }

    // Handle CommonJS require() calls
    fn visit_call_expr(&mut self, call: &CallExpr) {
        if let Callee::Expr(expr) = &call.callee {
            if let Expr::Ident(ident) = &**expr {
                if ident.sym.as_ref() == "require" {
                    if let Some(arg) = call.args.first() {
                        if let Expr::Lit(Lit::Str(s)) = &*arg.expr {
                            self.imports.push(s.value.to_string());
                        }
                    }
                }
            }
        }

        // Continue visiting children for nested requires
        call.visit_children_with(self);
    }

    // Handle dynamic imports: import('./foo')
    fn visit_expr(&mut self, expr: &Expr) {
        if let Expr::Call(call) = expr {
            if let Callee::Import(_) = &call.callee {
                if let Some(arg) = call.args.first() {
                    if let Expr::Lit(Lit::Str(s)) = &*arg.expr {
                        self.imports.push(s.value.to_string());
                    }
                }
            }
        }

        expr.visit_children_with(self);
    }
}
