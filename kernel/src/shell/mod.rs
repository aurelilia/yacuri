use crate::{
    drivers::{
        disk::fat::{fat_from_secondary, FatDir, FatFs},
        vga_buffer::{vga_buffer, Color},
    },
    print, println,
    shell::command::Command,
};
use alloc::{
    format,
    string::{String, ToString},
    vec,
    vec::Vec,
};
use core::cmp::{max, min};
use fatfs::{Error, IoBase, Read, Seek, SeekFrom, Write};
use lazy_static::lazy_static;
use pc_keyboard::{DecodedKey, KeyCode};
use spin::Mutex;

mod command;

lazy_static! {
    pub static ref SHELL: Mutex<Shell> = Mutex::new(Shell::new(fat_from_secondary()));
}

pub struct Shell {
    filesystem: FatFs,
    working_dir: Option<String>,
    current_command: String,
    cursor_pos: usize,
}

impl Shell {
    pub fn key_pressed(&mut self, key: DecodedKey) {
        match key {
            DecodedKey::Unicode('\x08') => {
                if self.cursor_at_end() {
                    self.current_command.pop();
                } else {
                    self.current_command.remove(self.cursor_pos - 1);
                }

                self.cursor_pos -= 1;
            }
            DecodedKey::Unicode('\n') => self.enter_pressed(),
            DecodedKey::Unicode(character) => {
                if self.cursor_at_end() {
                    self.current_command.push(character);
                } else {
                    self.current_command.insert(self.cursor_pos, character);
                }
                self.cursor_pos += 1;
            }

            DecodedKey::RawKey(KeyCode::ArrowLeft) => {
                self.cursor_pos = self.cursor_pos.checked_sub(1).unwrap_or(self.cursor_pos)
            }
            DecodedKey::RawKey(KeyCode::ArrowRight) => {
                self.cursor_pos = min(78, self.cursor_pos + 1)
            }

            DecodedKey::RawKey(key) => print!("{:?}", key),
        }
        self.redraw();
    }

    fn enter_pressed(&mut self) {
        vga_buffer(|w| w.set_color(Color::Yellow));
        println!("> {}", self.current_command);
        vga_buffer(|w| w.reset_color());

        let command = Command::from(&self.current_command);
        match command {
            Ok(Some(command)) => self.execute_command(command),
            Ok(None) => (),
            Err(msg) => println!("Failed to parse command: {}", msg),
        }

        self.current_command.clear();
        self.cursor_pos = 0;
    }

    fn execute_command(&mut self, command: Command) {
        match command {
            Command::Ls { directory } => {
                let dir = if let Some(directory) = directory {
                    self.workdir().open_dir(&directory)
                } else {
                    Ok(self.workdir())
                };

                if let Ok(dir) = dir {
                    let mut count = 0;
                    for r in dir.iter() {
                        let entry = r.unwrap();
                        println!("{}", entry.file_name());
                        count += 1;
                    }
                    println!("total {}", count)
                } else {
                    println!("ls: unknown directory")
                }
            }

            Command::Cat { file } => {
                let obj = self.workdir().open_file(&file);
                if let Ok(mut obj) = obj {
                    let size = obj.seek(SeekFrom::End(0)).unwrap();
                    let mut buf = Vec::with_capacity(size as usize);
                    unsafe {
                        buf.set_len(size as usize);
                    }

                    obj.seek(SeekFrom::Start(0));
                    let read = match obj.read(&mut buf) {
                        Ok(read) => read,
                        Err(err) => {
                            println!("cat: failed to read file: {:?}", err);
                            return;
                        }
                    };

                    let str = String::from_utf8(buf);
                    if let Ok(str) = str {
                        println!("{} ({} bytes):\n{}", file, read, str)
                    } else {
                        println!("cat: file is not valid UTF-8")
                    }
                }
            }

            Command::Cd { directory } => {
                let exists = self.workdir().open_dir(&directory).is_ok();
                match (exists, self.working_dir.clone()) {
                    (true, Some(workd)) => {
                        self.working_dir = Some(format!("{}/{}", workd, directory))
                    }
                    (true, None) => self.working_dir = Some(directory),
                    _ => println!("cd: unknown directory"),
                }
            }

            Command::Mkdir { directory } => {
                let res = self.workdir().create_dir(&directory);
                if let Err(err) = res {
                    println!("mkdir: failed to create directory: {:?}", err);
                }
            }

            Command::Put { file, text } => {
                let file = self.workdir().create_file(&file);
                if let Ok(mut file) = file {
                    let res = file.write_all(text.as_bytes());
                    if let Err(err) = res {
                        println!("put: failed to write file: {:?}", err);
                    }
                } else {
                    println!("put: failed to open file")
                }
            }

            Command::Exec { file } => println!("exec: error running {}: unimplemented", file),
        }
        println!();
    }

    fn workdir(&self) -> FatDir {
        if let Some(name) = &self.working_dir {
            self.filesystem.root_dir().open_dir(name).unwrap()
        } else {
            self.filesystem.root_dir()
        }
    }

    fn cursor_at_end(&self) -> bool {
        self.cursor_pos == self.current_command.len()
    }

    fn redraw(&mut self) {
        vga_buffer(|w| {
            w.set_cursor_x(self.cursor_pos);
            w.write_shell_line(&self.current_command);
        })
    }

    pub fn new(filesystem: FatFs) -> Shell {
        vga_buffer(|w| w.init_shell());
        Shell {
            filesystem,
            working_dir: None,
            current_command: "".to_string(),
            cursor_pos: 0,
        }
    }
}
