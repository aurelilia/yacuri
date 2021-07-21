use alloc::string::String;
use alloc::vec::Vec;
use fatfs::{Read, SeekFrom, Seek};
use spin::{RwLock, RwLockReadGuard};
use yacari::filesystem::{Filesystem, File};
use crate::drivers::disk::fat::{FatFile, FatDir};
use yacari::SmolStr;

pub mod ata_pio;
pub mod fat;

static FS_LOCK: RwLock<()> = RwLock::new(());

pub struct FileSystem<'fs> {
    fs: fat::FatFs,
    lock: RwLockReadGuard<'fs, ()>
}

impl<'fs> FileSystem<'fs> {
    pub fn new() -> Self {
        FileSystem { fs: fat::fat_from_secondary(), lock: FS_LOCK.read() }
    }
}

impl<'fs> Filesystem for FileSystem<'fs> {
    fn walk_directory<T: FnMut(File)>(&self, path: &str, mut cls: T) {
        let dir = self.fs.root_dir().open_dir(path).unwrap();
        walk_dir(dir, &mut Vec::new(), &mut cls)
    }
}

fn walk_dir<T: FnMut(File)>(entry: FatDir, path_buf: &mut Vec<SmolStr>, cls: &mut T) {
    for sub in entry.iter() {
        match sub {
            Ok(entry) if entry.is_dir() => {
                path_buf.push(SmolStr::new(entry.file_name()));
                walk_dir(entry.to_dir(), path_buf, cls);
                path_buf.pop();
            }

            Ok(entry) if entry.is_file() => {
                read_file(entry.to_file()).map(|contents| cls(File { path: path_buf.clone(), contents }));
            },

            _ => ()
        }
    }
}

fn read_file(mut file: FatFile) -> Option<String> {
    let size = file.seek(SeekFrom::End(0)).unwrap();
    let mut buf = Vec::with_capacity(size as usize);
    unsafe {
        buf.set_len(size as usize);
    }

    file.seek(SeekFrom::Start(0)).unwrap();
    file.read(&mut buf).unwrap();
    String::from_utf8(buf).ok()
}
