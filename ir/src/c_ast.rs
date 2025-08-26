use serde::{Deserialize, Serialize};
use std::{
    any::Any, fmt::Display, fs::File, path::{Path, PathBuf}, process::Command
};

use c2rust_transpile::c_ast::{ConversionContext, TypedAstContext};

use crate::{
    HarvestIR, Representation,
    raw_source::{RawDir, RawEntry},
};

#[derive(Debug)]
pub struct CAst {
    _ast: Vec<TypedAstContext>,
}

#[derive(Deserialize, Serialize, Debug, Default, Clone)]
struct CompileCmd {
    /// The working directory of the compilation. All paths specified in the command
    /// or file fields must be either absolute or relative to this directory.
    pub directory: PathBuf,
    /// The main translation unit source processed by this compilation step. This is
    /// used by tools as the key into the compilation database. There can be multiple
    /// command objects for the same file, for example if the same source file is compiled
    /// with different configurations.
    pub file: PathBuf,
}

fn populate_from(base: &Path) -> Vec<TypedAstContext> {
    let v: Vec<CompileCmd> = serde_json::from_reader(std::io::BufReader::new(File::open(base.join("compile_commands.json")).unwrap())).unwrap();
    v.iter().map(|cc| {
	ConversionContext::new(&c2rust_ast_exporter::get_untyped_ast(&cc.file, base, &[], false).unwrap()).typed_context
    }).collect()
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
	Some(Self {
	    _ast: populate_from(cc_dir.path())
        })
    }
}
