use x86_64::instructions::port::Port;
use fatfs::{Read, IoBase};
use crate::drivers::disk::ata_pio::IoPort::Status;
use crate::println;

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
    Read = 0x20
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

pub struct AtaBus {
    io_base: u16,
    control_base: u16,
    lba: usize,
    position: usize,
}

impl AtaBus {
    fn before_read_write(&self, sector_count: u8) {
        self.wait_status(StatusBits::BSY, false);
        self.io_write(IoPort::DriveSel, (0xE0 | ((self.lba >> 24) & 0xF)) as u8);
        self.io_write(IoPort::SectorCount, sector_count);
        self.io_write(IoPort::LbaLow, self.lba as u8);
        self.io_write(IoPort::LbaMid, (self.lba >> 8) as u8);
        self.io_write(IoPort::LbaHigh, (self.lba >> 16) as u8);
    }

    fn wait_ready(&self) {
        self.wait_status(StatusBits::BSY, false);
        self.wait_status(StatusBits::DRQ, true);
    }

    fn wait_status(&self, status: StatusBits, until: bool) {
        let mut port = self.io_port(IoPort::Status);
        while status.is_set(unsafe { port.read() }) != until {}
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

    fn min_required_sector_count(bytes: usize) -> u8 {
        (bytes / 512) as u8
    }

    pub fn new(io_base: u16, control_base: u16) -> AtaBus {
        let bus = AtaBus {
            io_base,
            control_base,
            lba: 0, // todo wat is lba??
            position: 0,
        };

        println!("{}", bus.io_read(IoPort::Status));
        // 0xFF = illegal value / floating bus, no drive attached
        assert_ne!(bus.io_read(IoPort::Status), 0xFF);
        bus
    }
}

impl IoBase for AtaBus {
    type Error = ();
}

impl Read for AtaBus {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        let sector_count = Self::min_required_sector_count(buf.len());
        self.before_read_write(sector_count);
        self.send_command(Command::Read);

        let mut data_port = self.io_port_16(IoPort::Data);
        for sector in 0..sector_count {
            self.wait_ready();
            for word in 0..256 {
                let read = unsafe { data_port.read() };
                let index = ((sector as usize * 256) + word) * 2;
                if index < buf.len() {
                    buf[index] = read as u8;
                    buf[index + 1] = (read >> 8) as u8;
                }
            }
        }

        Ok(buf.len())
    }
}
