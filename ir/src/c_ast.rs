use std::{
    any::Any,
    fmt::Display,
    path::Path,
    process::Command,
};

use c2rust_ast_exporter::clang_ast::AstContext;
use c2rust_transpile::c_ast::{ConversionContext, TypedAstContext};

use crate::{
    HarvestIR, Representation,
    raw_source::{RawDir, RawEntry},
};

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

impl Display for CAst {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl Representation for CAst {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl CAst {
    pub fn run_stage<'a>(ir: HarvestIR) -> Option<CAst> {
        for repr in ir.representations.values() {
            if let Some(r) = repr.as_any().downcast_ref::<RawDir>() {
                return Self::populate_from(r);
            }
        }
        None
    }

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

	// Copy source directory to the file system somewhere temporary
        let td = tempdir::TempDir::new("harvest").unwrap();
        reify(src, td.path()).ok()?;

	// Use cmake to generate a `compile_commands.json` file in a
	// separate build directory
        let cc_dir = tempdir::TempDir::new("harvest").unwrap();
        Command::new("cmake")
            .arg("-DCMAKE_EXPORT_COMPILE_COMMANDS=1")
            .arg("-S")
            .arg(td.path())
            .arg("-B")
            .arg(cc_dir.path())
            .output()
            .ok()?;
        populate_from(src, cc_dir.path(), td.path()).map(|ac| Self {
            _ast: ConversionContext::new(&ac).typed_context,
        })
    }
}
