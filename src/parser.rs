use log::warn;
use serde::Serialize;
use std::path::Path;
use swc_common::sync::Lrc;
use swc_common::{SourceFile, SourceMap, Span};
use swc_ecma_ast::{EsVersion, *};
use swc_ecma_parser::{lexer::Lexer, EsSyntax, Parser, StringInput, Syntax, TsSyntax};
use swc_ecma_visit::{noop_visit_type, Visit, VisitWith};

/// The kind of import statement
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum ImportKind {
    /// ES module import: `import { x } from './foo'`
    EsModule,
    /// CommonJS require: `require('./foo')`
    CommonJs,
    /// Dynamic import: `import('./foo')`
    Dynamic,
    /// Re-export: `export * from './foo'` or `export { x } from './foo'`
    ReExport,
}

impl std::fmt::Display for ImportKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ImportKind::EsModule => write!(f, "import"),
            ImportKind::CommonJs => write!(f, "require"),
            ImportKind::Dynamic => write!(f, "dynamic import"),
            ImportKind::ReExport => write!(f, "re-export"),
        }
    }
}

/// Information about a single import statement
#[derive(Debug, Clone, Serialize)]
pub struct ImportInfo {
    /// The import source/path (e.g., "./useUser")
    pub source: String,
    /// Line number (1-indexed)
    pub line: u32,
    /// The full import text (e.g., "import { useUser } from './useUser';")
    pub import_text: String,
    /// Whether this is a type-only import
    pub is_type_only: bool,
    /// The kind of import
    pub kind: ImportKind,
}

/// Options for import extraction
#[derive(Default, Clone)]
pub struct ParserOptions {
    /// If true, type-only imports (import type { Foo }) are excluded
    pub ignore_type_imports: bool,
}

/// Extracts import paths from a JavaScript/TypeScript file.
///
/// Parses the file and collects all import sources including:
/// - ES module imports (`import { x } from './foo'`)
/// - Re-exports (`export * from './foo'`)
/// - CommonJS requires (`require('./foo')`)
/// - Dynamic imports (`import('./foo')`)
///
/// Returns a Vec of ImportInfo with line numbers and import text.
pub fn get_imports_from_file(path: &Path, options: &ParserOptions) -> Vec<ImportInfo> {
    let (module, source_map, source_file) = match parse_file_to_ast(path) {
        Some(m) => m,
        None => return vec![],
    };

    let mut collector = ImportCollector::new(options.ignore_type_imports, source_map, source_file);
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

fn parse_file_to_ast(path: &Path) -> Option<(Module, Lrc<SourceMap>, Lrc<SourceFile>)> {
    let cm = Lrc::new(SourceMap::default());
    let fm = cm.load_file(path).ok()?;

    let syntax = get_syntax_for_file(path);
    let lexer = Lexer::new(syntax, EsVersion::Es2022, StringInput::from(&*fm), None);
    let mut parser = Parser::new_from(lexer);

    match parser.parse_module() {
        Ok(module) => Some((module, cm, fm)),
        Err(err) => {
            warn!(
                "Failed to parse '{}': {}",
                path.display(),
                format!("{:?}", err)
                    .lines()
                    .next()
                    .unwrap_or("Unknown error")
            );
            None
        }
    }
}

struct ImportCollector {
    imports: Vec<ImportInfo>,
    ignore_type_imports: bool,
    source_map: Lrc<SourceMap>,
    source_file: Lrc<SourceFile>,
}

impl ImportCollector {
    fn new(
        ignore_type_imports: bool,
        source_map: Lrc<SourceMap>,
        source_file: Lrc<SourceFile>,
    ) -> Self {
        Self {
            imports: vec![],
            ignore_type_imports,
            source_map,
            source_file,
        }
    }

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
        let all_type_only = import_decl.specifiers.iter().all(|spec| match spec {
            ImportSpecifier::Named(named) => named.is_type_only,
            // Default and namespace imports are value imports
            ImportSpecifier::Default(_) | ImportSpecifier::Namespace(_) => false,
        });

        // If all specifiers are type-only, skip this import
        !all_type_only
    }

    /// Extract line number from a span
    fn get_line(&self, span: Span) -> u32 {
        let loc = self.source_map.lookup_char_pos(span.lo);
        loc.line as u32
    }

    /// Extract the source text for a span
    fn get_span_text(&self, span: Span) -> String {
        let lo = span.lo.0 as usize - self.source_file.start_pos.0 as usize;
        let hi = span.hi.0 as usize - self.source_file.start_pos.0 as usize;
        let src = self.source_file.src.as_ref();

        if lo <= hi && hi <= src.len() {
            src[lo..hi].to_string()
        } else {
            String::new()
        }
    }

    fn add_import(&mut self, source: String, span: Span, is_type_only: bool, kind: ImportKind) {
        let line = self.get_line(span);
        let import_text = self.get_span_text(span);

        self.imports.push(ImportInfo {
            source,
            line,
            import_text,
            is_type_only,
            kind,
        });
    }
}

impl Visit for ImportCollector {
    noop_visit_type!();

    fn visit_module_item(&mut self, item: &ModuleItem) {
        match item {
            // ES Module imports: import { foo } from './foo'
            ModuleItem::ModuleDecl(ModuleDecl::Import(import_decl)) => {
                if self.should_include_import(import_decl) {
                    self.add_import(
                        import_decl.src.value.to_string(),
                        import_decl.span,
                        import_decl.type_only,
                        ImportKind::EsModule,
                    );
                }
            }
            // Re-exports: export * from './foo'
            ModuleItem::ModuleDecl(ModuleDecl::ExportAll(export_all)) => {
                // export type * from './foo' is type-only
                if !(self.ignore_type_imports && export_all.type_only) {
                    self.add_import(
                        export_all.src.value.to_string(),
                        export_all.span,
                        export_all.type_only,
                        ImportKind::ReExport,
                    );
                }
            }
            // Named re-exports: export { foo } from './foo'
            ModuleItem::ModuleDecl(ModuleDecl::ExportNamed(named_export)) => {
                if let Some(src) = &named_export.src {
                    // Check if this is a type-only export
                    if !(self.ignore_type_imports && named_export.type_only) {
                        self.add_import(
                            src.value.to_string(),
                            named_export.span,
                            named_export.type_only,
                            ImportKind::ReExport,
                        );
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
                            self.add_import(
                                s.value.to_string(),
                                call.span,
                                false,
                                ImportKind::CommonJs,
                            );
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
                        self.add_import(s.value.to_string(), call.span, false, ImportKind::Dynamic);
                    }
                }
            }
        }

        expr.visit_children_with(self);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn create_temp_file(content: &str, extension: &str) -> NamedTempFile {
        let mut file = tempfile::Builder::new()
            .suffix(extension)
            .tempfile()
            .unwrap();
        file.write_all(content.as_bytes()).unwrap();
        file
    }

    #[test]
    fn test_parse_es_import() {
        let file = create_temp_file("import { foo } from './bar';", ".ts");
        let imports = get_imports_from_file(file.path(), &ParserOptions::default());

        assert_eq!(imports.len(), 1);
        assert_eq!(imports[0].source, "./bar");
        assert_eq!(imports[0].line, 1);
        assert_eq!(imports[0].kind, ImportKind::EsModule);
        assert!(!imports[0].is_type_only);
    }

    #[test]
    fn test_parse_type_only_import() {
        let file = create_temp_file("import type { Foo } from './types';", ".ts");

        // Without ignore_type_imports
        let imports = get_imports_from_file(file.path(), &ParserOptions::default());
        assert_eq!(imports.len(), 1);
        assert!(imports[0].is_type_only);

        // With ignore_type_imports
        let imports = get_imports_from_file(
            file.path(),
            &ParserOptions {
                ignore_type_imports: true,
            },
        );
        assert_eq!(imports.len(), 0);
    }

    #[test]
    fn test_parse_require() {
        let file = create_temp_file("const foo = require('./bar');", ".js");
        let imports = get_imports_from_file(file.path(), &ParserOptions::default());

        assert_eq!(imports.len(), 1);
        assert_eq!(imports[0].source, "./bar");
        assert_eq!(imports[0].kind, ImportKind::CommonJs);
    }

    #[test]
    fn test_parse_dynamic_import() {
        let file = create_temp_file("const foo = import('./bar');", ".ts");
        let imports = get_imports_from_file(file.path(), &ParserOptions::default());

        assert_eq!(imports.len(), 1);
        assert_eq!(imports[0].source, "./bar");
        assert_eq!(imports[0].kind, ImportKind::Dynamic);
    }

    #[test]
    fn test_parse_reexport() {
        let file = create_temp_file("export * from './utils';", ".ts");
        let imports = get_imports_from_file(file.path(), &ParserOptions::default());

        assert_eq!(imports.len(), 1);
        assert_eq!(imports[0].source, "./utils");
        assert_eq!(imports[0].kind, ImportKind::ReExport);
    }

    #[test]
    fn test_line_numbers() {
        let file = create_temp_file(
            "// comment\nimport { a } from './a';\nimport { b } from './b';",
            ".ts",
        );
        let imports = get_imports_from_file(file.path(), &ParserOptions::default());

        assert_eq!(imports.len(), 2);
        assert_eq!(imports[0].line, 2);
        assert_eq!(imports[1].line, 3);
    }

    #[test]
    fn test_import_text_captured() {
        let file = create_temp_file("import { foo, bar } from './baz';", ".ts");
        let imports = get_imports_from_file(file.path(), &ParserOptions::default());

        assert_eq!(imports.len(), 1);
        assert_eq!(imports[0].import_text, "import { foo, bar } from './baz';");
    }
}
