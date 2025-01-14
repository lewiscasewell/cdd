use std::path::Path;
use std::rc::Rc;
use swc_common::SourceMap;
use swc_ecma_ast::{EsVersion, *};
use swc_ecma_parser::{lexer::Lexer, Parser, StringInput, Syntax, TsSyntax};
use swc_ecma_visit::{noop_visit_type, Visit, VisitWith};

pub fn get_imports_from_file(path: &Path) -> Vec<String> {
    let module = match parse_file_to_ast(path) {
        Some(m) => m,
        None => return vec![],
    };

    let mut collector = ImportCollector { imports: vec![] };
    module.visit_with(&mut collector);
    collector.imports
}

fn parse_file_to_ast(path: &Path) -> Option<Module> {
    let cm = Rc::new(SourceMap::default());
    let fm = cm.load_file(path).ok()?;

    let syntax = Syntax::Typescript(TsSyntax {
        tsx: true,
        decorators: true,
        dts: false,
        no_early_errors: false,
        ..Default::default()
    });

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
}

impl Visit for ImportCollector {
    noop_visit_type!();

    fn visit_module_item(&mut self, item: &ModuleItem) {
        match item {
            ModuleItem::ModuleDecl(ModuleDecl::Import(import_decl)) => {
                self.imports.push(import_decl.src.value.to_string());
            }
            ModuleItem::ModuleDecl(ModuleDecl::ExportAll(export_all)) => {
                self.imports.push(export_all.src.value.to_string());
            }
            ModuleItem::ModuleDecl(ModuleDecl::ExportNamed(named_export)) => {
                if let Some(src) = &named_export.src {
                    self.imports.push(src.value.to_string());
                }
            }
            _ => {}
        }

        item.visit_children_with(self);
    }
}
