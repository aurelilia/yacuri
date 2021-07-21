use crate::smol_str::SmolStr;
use alloc::{string::String, vec::Vec};

pub struct File {
    pub path: Vec<SmolStr>,
    pub contents: String,
}

pub trait Filesystem {
    fn walk_directory<T: FnMut(File)>(&self, path: &str, cls: T);
}

#[cfg(feature = "std")]
pub mod os_fs {
    use super::File as YFile;
    use crate::{filesystem::Filesystem, smol_str::SmolStr};
    use alloc::vec::Vec;
    use std::{fs, path::PathBuf};

    pub struct OsFs;
    impl Filesystem for OsFs {
        fn walk_directory<T: FnMut(YFile)>(&self, path: &str, mut cls: T) {
            let dir = PathBuf::from(path);
            let mut path = Vec::with_capacity(5);
            walk_file(dir, &mut path, &mut cls)
        }
    }

    fn walk_file<T: FnMut(YFile)>(input: PathBuf, path: &mut Vec<SmolStr>, cls: &mut T) {
        path.push(stem_to_smol(&input));
        if let Ok(dir) = input.read_dir() {
            for file in dir {
                let file = file.expect("Failed to read file").path();
                walk_file(file, path, cls)
            }
        } else if *input
            .extension()
            .map(|ext| ext == "yac")
            .get_or_insert(false)
        {
            cls(YFile {
                path: path.clone(),
                contents: fs::read_to_string(&input).expect("Failed to read file."),
            });
        }
    }

    pub fn stem_to_smol(path: &PathBuf) -> SmolStr {
        SmolStr::new(path.file_stem().unwrap().to_str().unwrap())
    }
}
