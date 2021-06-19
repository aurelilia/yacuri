use x86_64::instructions::port::Port;
use fatfs::{Read, IoBase, Write, Seek, SeekFrom};
use crate::drivers::disk::ata_pio::IoPort::Status;
use crate::{serial_println, println};

#[repr(u8)]
#[derive(Copy, Clone)]
enum StatusBits {
    BSY = 0x80,
    RDY = 0x40,
    DRQ = 0x08,
    DF = 0x20,
    ERR = 0x01,
}

impl StatusBits {
    fn is_set(self, val: u8) -> bool {
        val & self as u8 != 0
    }
}

#[repr(u8)]
enum Command {
    Read = 0x20,
    Write = 0x30,
    CacheFlush = 0xE7
}

#[repr(C)]
enum IoPort {
    Data,
    ErrFeatures,
    SectorCount,
    LbaLow,
    LbaMid,
    LbaHigh,
    DriveSel,
    Status,
}

#[repr(C)]
enum ControlPort {
    Status,
    Address,
}

pub struct AtaDrive {
    io_base: u16,
    control_base: u16,
    position: usize,
}

impl AtaDrive {
    fn before_read_write(&self, sector_count: u8) {
        let lba = self.calc_lba();
        self.wait_status(StatusBits::BSY, false);
        self.io_write(IoPort::DriveSel, (0xF0 | ((lba >> 24) & 0xF)) as u8);
        self.io_write(IoPort::SectorCount, sector_count);
        self.io_write(IoPort::LbaLow, lba as u8);
        self.io_write(IoPort::LbaMid, (lba >> 8) as u8);
        self.io_write(IoPort::LbaHigh, (lba >> 16) as u8);
    }

    fn get_partial_write_sectors(&mut self, len: usize) -> (Option<[u16; 256]>, Option<[u16; 256]>) {
        let start = self.read_sector_if_unaligned();
        self.position += len;
        let end = self.read_sector_if_unaligned();
        self.position -= len;
        (start, end)
    }

    fn read_sector_if_unaligned(&self) -> Option<[u16; 256]> {
        if !self.pos_aligned() {
            Some(self.read_sector())
        } else {
            None
        }
    }

    fn read_sector(&self) -> [u16; 256] {
        self.before_read_write(1);
        self.send_command(Command::Read);

        let mut data_port = self.io_port_16(IoPort::Data);
        let mut buf = [0; 256];
        self.wait_ready();
        for word in 0..256 {
            let read = unsafe { data_port.read() };
            buf[word] = read;
        }
        buf
    }

    fn wait_ready(&self) {
        self.wait_status(StatusBits::BSY, false);
        self.wait_status(StatusBits::DRQ, true);
    }

    fn wait_status(&self, status: StatusBits, until: bool) {
        let mut port = self.io_port(IoPort::Status);
        while status.is_set(unsafe { port.read() }) != until {}
    }

    fn calc_lba(&self) -> usize {
        (self.position / 512) as usize
    }

    fn send_command(&self, command: Command) {
        self.io_write(IoPort::Status, command as u8);
    }

    fn io_read(&self, io_port: IoPort) -> u8 {
        unsafe { self.io_port(io_port).read() }
    }

    fn io_write(&self, io_port: IoPort, value: u8) {
        unsafe { self.io_port(io_port).write(value); }
    }

    fn io_port(&self, io_port: IoPort) -> Port<u8> {
        Port::new(self.io_base + io_port as u16)
    }

    fn io_port_16(&self, io_port: IoPort) -> Port<u16> {
        Port::new(self.io_base + io_port as u16)
    }

    fn con_port(&self, control_port: ControlPort) -> Port<u8> {
        Port::new(self.control_base + control_port as u16)
    }

    fn min_required_sector_count(&self, bytes: usize) -> u8 {
        let sector_aligned = Self::is_sector_aligned(bytes) && bytes != 0;
        let bleeds_into_next = if sector_aligned {
            !self.pos_aligned()
        } else {
            (self.position & 511) + (bytes & 511) > 512
        };
        ((bytes / 512) as u8) + bleeds_into_next as u8 + !sector_aligned as u8
    }

    fn pos_aligned(&self) -> bool {
        Self::is_sector_aligned(self.position as usize)
    }

    fn is_sector_aligned(value: usize) -> bool {
        value & 511 == 0
    }

    pub fn new(io_base: u16, control_base: u16) -> AtaDrive {
        let bus = AtaDrive {
            io_base,
            control_base,
            position: 0,
        };

        // 0xFF = illegal value / floating bus, no drive attached
        assert_ne!(bus.io_read(IoPort::Status), 0xFF);
        bus
    }
}

impl IoBase for AtaDrive {
    type Error = ();
}

impl Read for AtaDrive {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        let sector_count = self.min_required_sector_count(buf.len());
        self.before_read_write(sector_count);
        self.send_command(Command::Read);

        let mut data_port = self.io_port_16(IoPort::Data);
        let sector_offset = (self.position % 512) as i64;
        for sector in 0..sector_count {
            self.wait_ready();
            for word in 0..256 {
                let read = unsafe { data_port.read() };

                let index: i64 = (((sector as i64 * 256) + word) * 2) - sector_offset;
                let i = index as usize;
                let buf_len = buf.len() as i64;

                match () {
                    _ if index == -1 => buf[0] = (read >> 8) as u8,
                    _ if index < -1 => (),
                    _ if (index + 1) < buf_len => {
                        buf[i] = read as u8;
                        buf[i + 1] = (read >> 8) as u8;
                    }
                    _ if index < buf_len => buf[i] = read as u8,
                    _ => ()
                }
            }
        }

        self.position += buf.len();
        Ok(buf.len())
    }
}

impl Write for AtaDrive {
    fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        let sector_count = self.min_required_sector_count(buf.len());
        let (start_sector, end_sector) = self.get_partial_write_sectors(buf.len());
        self.before_read_write(sector_count);
        self.send_command(Command::Write);

        let mut data_port = self.io_port_16(IoPort::Data);
        let sector_offset = (self.position % 512) as i64;
        for sector in 0..sector_count {
            self.wait_ready();
            for word in 0..256usize {
                let index: i64 = (((sector as i64 * 256) + word as i64) * 2) - sector_offset;
                let i = index as usize;
                let buf_len = buf.len() as i64;

                let to_write = match () {
                    _ if index == -1 => (start_sector.unwrap()[word] & 255) + ((buf[0] as u16) << 8),
                    _ if index < -1 => start_sector.unwrap()[word],
                    _ if (index + 1) < buf_len => buf[i] as u16 + ((buf[i + 1] as u16) << 8),
                    _ if index < buf_len => buf[i] as u16 + (end_sector.unwrap()[word] & 0xFF00),
                    _ => end_sector.unwrap()[word]
                };
                unsafe { data_port.write(to_write) }
            }
        }

        self.send_command(Command::CacheFlush);
        self.position += buf.len();
        Ok(buf.len())
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}

impl Seek for AtaDrive {
    fn seek(&mut self, pos: SeekFrom) -> Result<u64, Self::Error> {
        match pos {
            SeekFrom::Start(pos) => {
                self.position = pos as usize;
                Ok(pos)
            }

            SeekFrom::Current(by) => {
                let res = (self.position as i64 + by);
                if res >= 0 {
                    self.position = res as usize;
                    Ok(self.position as u64)
                } else {
                    Err(())
                }
            }

            _ => Err(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::AtaDrive;
    use crate::serial_println;
    use fatfs::{Seek, SeekFrom, Read, Write};
    use spin::{Mutex, MutexGuard};
    use lazy_static::lazy_static;
    use rand::rngs::SmallRng;
    use rand::{SeedableRng, RngCore};

    // 64KiB drive read from disk, this is what AtaBus should return.
    static ACTUAL: &'static [u8; 1024 * 64] = include_bytes!("test_drive.bin");
    // The bus used for all tests.
    lazy_static! {
        pub static ref BUS: Mutex<AtaDrive> = Mutex::new(AtaDrive::new(0x1F0, 0x3F6));
    }

    #[test_case]
    fn seek() {
        let mut bus = init();
        bus.seek(SeekFrom::Start(12));
        assert_eq!(bus.position, 12);
        bus.seek(SeekFrom::Current(12));
        assert_eq!(bus.position, 24);
        bus.seek(SeekFrom::Start(457));
        assert_eq!(bus.position, 457);
        bus.seek(SeekFrom::Current(-12));
        assert_eq!(bus.position, 445);

        assert_eq!(bus.seek(SeekFrom::Current(-1000)), Err(()));
        assert_eq!(bus.seek(SeekFrom::End(0)), Err(()));
    }

    #[test_case]
    fn correct_sector_count() {
        let mut bus = init();
        assert_eq!(bus.min_required_sector_count(3), 1);
        assert_eq!(bus.min_required_sector_count(200), 1);
        assert_eq!(bus.min_required_sector_count(512), 1);
        assert_eq!(bus.min_required_sector_count(513), 2);
        assert_eq!(bus.min_required_sector_count(2000), 4);

        bus.seek(SeekFrom::Start(200));
        assert_eq!(bus.min_required_sector_count(3), 1);
        assert_eq!(bus.min_required_sector_count(200), 1);
        assert_eq!(bus.min_required_sector_count(512), 2);
        assert_eq!(bus.min_required_sector_count(513), 2);
        assert_eq!(bus.min_required_sector_count(2000), 5);
    }

    #[test_case]
    fn read_first_sector() {
        read_count::<512>(1)
    }

    #[test_case]
    fn read_multiple_sectors() {
        read_count::<2048>(1)
    }

    #[test_case]
    fn read_multiple_chunked() {
        read_count::<512>(10)
    }

    #[test_case]
    fn read_partial_sector() {
        read_count::<128>(1)
    }

    #[test_case]
    fn read_partial_multiple() {
        read_count::<128>(10)
    }

    #[test_case]
    fn read_non_pow2() {
        read_count::<200>(10)
    }

    #[test_case]
    fn read_uneven() {
        read_count::<3>(10);
        read_count::<201>(10)
    }

    fn read_count<const COUNT: usize>(repetitions: usize) {
        let mut bus = init();
        let mut buf = [0; COUNT];

        for i in 0..repetitions {
            bus.read(&mut buf);
            let buf_start = i * COUNT;
            assert_eq!(buf, ACTUAL[buf_start..(buf_start + COUNT)]);
        }
    }

    #[test_case]
    fn write_preserve() {
        let mut bus = init();
        let mut rng = SmallRng::seed_from_u64(679);
        let mut write_buf = [35; 100];
        let mut verify_buf = [0; 512];

        bus.seek(SeekFrom::Start(100));
        bus.write(&write_buf);
        bus.seek(SeekFrom::Start(0));
        bus.read(&mut verify_buf);
        assert_eq!(verify_buf[0..100], ACTUAL[0..100]);
        assert_eq!(verify_buf[100..200], write_buf);
        assert_eq!(verify_buf[200..512], ACTUAL[200..512]);
    }

    #[test_case]
    fn write_first_sector() {
        write_verify::<512>(1, 124254)
    }

    #[test_case]
    fn write_multiple_sectors() {
        write_verify::<2048>(1, 096789)
    }

    #[test_case]
    fn write_multiple_chunked() {
        write_verify::<512>(10, 45897689)
    }

    #[test_case]
    fn write_partial_sector() {
        write_verify::<128>(1, 42)
    }

    #[test_case]
    fn write_partial_multiple() {
        write_verify::<128>(10, 42069)
    }

    #[test_case]
    fn write_non_pow2() {
        write_verify::<200>(10, 20)
    }

    #[test_case]
    fn write_uneven() {
        write_verify::<3>(10, 123);
        write_verify::<201>(10, 12123)
    }

    fn write_verify<const COUNT: usize>(repetitions: usize, seed: u64) {
        let mut bus = init();
        let mut rng = SmallRng::seed_from_u64(seed);
        let mut write_buf = [0; COUNT];
        let mut verify_buf = [0; COUNT];

        for i in 0..repetitions {
            for elem in &mut write_buf {
                *elem = rng.next_u32() as u8;
            }

            bus.write(&write_buf);
            bus.seek(SeekFrom::Current(-(COUNT as i64)));
            bus.read(&mut verify_buf);
            assert_eq!(write_buf, verify_buf);
        }
    }

    fn init() -> MutexGuard<'static, AtaDrive> {
        let mut bus: MutexGuard<AtaDrive> = BUS.lock();
        bus.seek(SeekFrom::Start(0));
        bus
    }
}
