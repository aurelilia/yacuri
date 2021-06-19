use fatfs::{FileSystem, DefaultTimeProvider, LossyOemCpConverter};
use crate::drivers::disk::ata_pio::AtaDrive;

pub type FatFs = FileSystem<AtaDrive, DefaultTimeProvider, LossyOemCpConverter>;

pub fn fat_from_ata(ata: AtaDrive) -> FatFs {
    FatFs::new(ata, fatfs::FsOptions::new()).expect("Failed to create FAT fs")
}

pub fn fat_from_secondary() -> FatFs {
    let secondary = AtaDrive::new(0x1F0, 0x3F6);
    fat_from_ata(secondary)
}