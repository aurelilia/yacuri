use crate::drivers::disk::ata_pio::AtaDrive;
use fatfs::{DefaultTimeProvider, Dir, DirEntry, File, FileSystem, LossyOemCpConverter};

pub type FatFs = FileSystem<AtaDrive, DefaultTimeProvider, LossyOemCpConverter>;
pub type FatDir<'d> = Dir<'d, AtaDrive, DefaultTimeProvider, LossyOemCpConverter>;
pub type FatFile<'d> = File<'d, AtaDrive, DefaultTimeProvider, LossyOemCpConverter>;
pub type FatEntry<'d> = DirEntry<'d, AtaDrive, DefaultTimeProvider, LossyOemCpConverter>;

/// Treat a given block device as a FAT filesystem.
///
/// # Safety
/// This function will panic if the given block device is not FAT-formatted.
/// It should only be called once.
fn fat_from_ata(ata: AtaDrive) -> FatFs {
    FatFs::new(ata, fatfs::FsOptions::new()).expect("Failed to create FAT fs")
}

/// Treat the secondary block device attached to the primary controller as a FAT filesystem.
pub fn fat_from_secondary() -> FatFs {
    let secondary = unsafe { AtaDrive::new(0x1F0, 0x3F6) };
    fat_from_ata(secondary)
}
