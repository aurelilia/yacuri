use core::{fmt, fmt::Write};
use lazy_static::lazy_static;
use spin::Mutex;
use volatile::Volatile;
use x86_64::instructions::{interrupts, port::Port};

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Color {
    Black = 0,
    Blue = 1,
    Green = 2,
    Cyan = 3,
    Red = 4,
    Magenta = 5,
    Brown = 6,
    LightGray = 7,
    DarkGray = 8,
    LightBlue = 9,
    LightGreen = 10,
    LightCyan = 11,
    LightRed = 12,
    Pink = 13,
    Yellow = 14,
    White = 15,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
struct ColorCode(u8);

impl ColorCode {
    fn new(foreground: Color, background: Color) -> ColorCode {
        ColorCode((background as u8) << 4 | (foreground as u8))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
struct ScreenChar {
    ascii_character: u8,
    color_code: ColorCode,
}

const BUFFER_HEIGHT: usize = 25;
const BUFFER_WIDTH: usize = 80;

#[repr(transparent)]
struct Buffer {
    chars: [[Volatile<ScreenChar>; BUFFER_WIDTH]; BUFFER_HEIGHT],
}

struct Cursor {
    port1: Port<u8>,
    port2: Port<u8>,
}

pub struct Writer {
    row_position: usize,
    column_position: usize,
    color_code: ColorCode,
    buffer: &'static mut Buffer,
    cursor: Cursor,
}

impl Writer {
    pub fn write_string(&mut self, s: &str) {
        for byte in s.bytes() {
            match byte {
                // printable ASCII byte or newline
                0x20..=0x7e | b'\n' => self.write_byte(byte),
                // not part of printable ASCII range
                _ => self.write_byte(0xfe),
            }
        }
    }

    pub fn write_byte(&mut self, byte: u8) {
        match byte {
            b'\n' => self.new_line(),
            byte => {
                if self.column_position >= BUFFER_WIDTH {
                    self.new_line();
                }

                self.buffer.chars[self.row_position][self.column_position].write(ScreenChar {
                    ascii_character: byte,
                    color_code: self.color_code,
                });
                self.shift_by(1);
            }
        }
    }

    fn new_line(&mut self) {
        self.shift_rows_up(1);
        self.column_position = 0;
        self.update_cursor();
    }

    fn shift_rows_up(&mut self, by: usize) {
        for row in by..BUFFER_HEIGHT {
            for col in 0..BUFFER_WIDTH {
                let character = self.buffer.chars[row][col].read();
                self.buffer.chars[row - by][col].write(character);
            }
        }
        for row in (BUFFER_HEIGHT - by)..BUFFER_HEIGHT {
            self.clear_row(row)
        }
    }

    fn remove_current_char(&mut self) {
        self.shift_by(-1);
        self.buffer.chars[self.row_position][self.column_position].write(ScreenChar {
            ascii_character: b' ',
            color_code: self.color_code,
        });
    }

    fn shift_by(&mut self, by: isize) {
        let res = self.column_position as isize + by;
        if res < 0 {
            self.row_position -= 1;
            self.column_position = BUFFER_WIDTH - 1;
            while self.read_at_current() == b' ' && self.column_position > 1 {
                self.column_position -= 1;
            }
            self.column_position += 1;
        } else if res >= BUFFER_WIDTH as isize {
            self.new_line();
        } else {
            self.column_position = res as usize;
        }

        self.update_cursor();
    }

    fn shift_row_by(&mut self, by: isize) {
        let res = self.row_position as isize + by;
        if res < 0 {
            self.row_position = 0;
        } else if res >= BUFFER_HEIGHT as isize {
            self.row_position = BUFFER_HEIGHT - 1;
        } else {
            self.row_position = res as usize;
        }

        self.update_cursor();
    }

    fn clear_row(&mut self, row: usize) {
        let blank = ScreenChar {
            ascii_character: b' ',
            color_code: self.color_code,
        };
        for col in 0..BUFFER_WIDTH {
            self.buffer.chars[row][col].write(blank);
        }
    }

    fn read_at_current(&self) -> u8 {
        self.buffer.chars[self.row_position][self.column_position]
            .read()
            .ascii_character
    }

    fn update_cursor(&mut self) {
        let position = self.row_position * BUFFER_WIDTH + self.column_position;
        unsafe {
            self.cursor.port1.write(0x0F);
            self.cursor.port2.write((position & 0xFF) as u8);
            self.cursor.port1.write(0x0E);
            self.cursor.port2.write(((position >> 8) & 0xFF) as u8);
        }
    }
}

impl fmt::Write for Writer {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.write_string(s);
        Ok(())
    }
}

lazy_static! {
    pub static ref WRITER: Mutex<Writer> = Mutex::new(Writer {
        row_position: BUFFER_HEIGHT - 1,
        column_position: 0,
        color_code: ColorCode::new(Color::Magenta, Color::Black),
        buffer: unsafe { &mut *(0xb8000 as *mut Buffer) },
        cursor: Cursor {
            port1: Port::new(0x3D4),
            port2: Port::new(0x3D5)
        }
    });
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::drivers::vga_buffer::_print(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}

#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    interrupts::without_interrupts(|| WRITER.lock().write_fmt(args).unwrap());
}

pub fn clear_last_char() {
    interrupts::without_interrupts(|| WRITER.lock().remove_current_char());
}

pub fn shift_column(by: isize) {
    interrupts::without_interrupts(|| WRITER.lock().shift_by(by));
}

pub fn shift_row(by: isize) {
    interrupts::without_interrupts(|| WRITER.lock().shift_row_by(by));
}

#[cfg(test)]
mod tests {
    use super::{BUFFER_HEIGHT, WRITER};

    #[test_case]
    fn test_println_simple() {
        println!("test_println_simple output");
    }

    #[test_case]
    fn test_println_many() {
        for _ in 0..200 {
            println!("test_println_many output");
        }
    }

    #[test_case]
    fn test_println_output() {
        use core::fmt::Write;
        use x86_64::instructions::interrupts;

        let s = "Some test string that fits on a single line";
        interrupts::without_interrupts(|| {
            let mut writer = WRITER.lock();
            writeln!(writer, "\n{}", s).expect("writeln failed");
            for (i, c) in s.chars().enumerate() {
                let screen_char = writer.buffer.chars[BUFFER_HEIGHT - 2][i].read();
                assert_eq!(char::from(screen_char.ascii_character), c);
            }
        });
    }
}