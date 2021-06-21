use crate::{
    drivers::{
        disk::fat::{FatDir, FatFs},
        vga_buffer::{vga_buffer, Color},
    },
    kprintln, print, println,
    shell::command::Command,
    QemuExitCode,
};
use alloc::{
    format,
    string::{String, ToString},
    vec::Vec,
};
use core::cmp::min;
use fatfs::{Read, Seek, SeekFrom, Write};
use pc_keyboard::{DecodedKey, KeyCode};

mod command;

pub struct Shell {
    filesystem: Option<FatFs>,
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
                let content = self.read_file(&file);
                if let Some(content) = content {
                    println!("{} ({} bytes):\n{}", file, content.len(), content)
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

            Command::Exec { file } => {
                let file = self.read_file(&file);
                if let Some(file) = file {
                    println!("executing {} ({} bytes)...", file, file.len());
                    kprintln!("{:#?}", yacari::execute_program(&file))
                }
            }

            Command::Exit => {
                self.filesystem.take().unwrap().unmount().unwrap();
                crate::exit_qemu(QemuExitCode::Success);
            }
        }
        println!();
    }

    fn read_file(&mut self, rel_path: &str) -> Option<String> {
        let obj = self.workdir().open_file(&rel_path);
        if let Ok(mut obj) = obj {
            let size = obj.seek(SeekFrom::End(0)).unwrap();
            let mut buf = Vec::with_capacity(size as usize);
            unsafe {
                buf.set_len(size as usize);
            }

            obj.seek(SeekFrom::Start(0)).unwrap();
            match obj.read(&mut buf) {
                Ok(_) => (),
                Err(err) => {
                    println!("failed to read file: {:?}", err);
                    return None;
                }
            };

            let str = String::from_utf8(buf);
            if let Ok(str) = str {
                Some(str)
            } else {
                println!("error: file is not valid UTF-8");
                None
            }
        } else {
            println!("error: file does not exist");
            None
        }
    }

    fn workdir(&self) -> FatDir {
        if let Some(name) = &self.working_dir {
            self.filesystem
                .as_ref()
                .unwrap()
                .root_dir()
                .open_dir(name)
                .unwrap()
        } else {
            self.filesystem.as_ref().unwrap().root_dir()
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
            filesystem: Some(filesystem),
            working_dir: None,
            current_command: "".to_string(),
            cursor_pos: 0,
        }
    }
}
