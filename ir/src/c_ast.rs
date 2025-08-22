use std::path::Path;

use c2rust_ast_exporter::clang_ast::AstContext;
use c2rust_transpile::c_ast::{ConversionContext, TypedAstContext};

use crate::raw_source::{RawDir, RawEntry};

#[derive(Debug)]
pub struct CAst {
    _ast: TypedAstContext,
}

fn populate_from(src: &RawDir, base: &Path, prefix: &Path) -> Option<AstContext> {
    for (name, entry) in src.0.iter() {
        let full_path = prefix.join(name);
        match entry {
            RawEntry::File(_) => {
                if !name.as_encoded_bytes().ends_with(b".c") {
                    continue;
                }
                let untyped_ast =
                    c2rust_ast_exporter::get_untyped_ast(&full_path, base, &[], false).unwrap();
                return Some(untyped_ast);
            }
            RawEntry::Dir(subdir) => {
                if let Some(res) = populate_from(subdir, base, &prefix.join(name)) {
                    return Some(res);
                }
            }
        }
    }
    None
}

impl CAst {
    pub fn populate_from(src: &RawDir) -> Option<CAst> {
        fn reify(src: &RawDir, dir: &Path) -> std::io::Result<()> {
            for (name, entry) in src.0.iter() {
                match entry {
                    RawEntry::File(contents) => {
                        std::fs::write(dir.join(name), contents).unwrap();
                    }
                    RawEntry::Dir(subdir) => {
                        std::fs::create_dir(dir.join(name))?;
                        reify(subdir, &dir.join(name))?;
                    }
                }
            }
            Ok(())
        }

        let td = tempdir::TempDir::new("harvest").unwrap();
        reify(src, td.path()).ok()?;
        populate_from(src, td.path(), td.path()).map(|ac| Self {
            _ast: ConversionContext::new(&ac).typed_context,
        })
    }
}
