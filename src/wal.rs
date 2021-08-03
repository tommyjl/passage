use crate::command::Command;
use crate::objects::parse;
use crate::server::MESSAGE_MAX_SIZE;
use nix::unistd::fsync;
use std::convert::TryFrom;
use std::fs::{File, OpenOptions};
use std::io::prelude::*;
use std::io::{Cursor, Result};
use std::os::unix::io::AsRawFd;
use std::sync::Mutex;

pub struct Wal {
    file: Mutex<File>,
}

impl Wal {
    pub fn new() -> Result<Self> {
        let path = "./wal.txt";
        let file = Mutex::new(
            OpenOptions::new()
                .create(true)
                .read(true)
                .append(true)
                .open(path)?,
        );
        Ok(Self { file })
    }

    pub fn read(&self) -> Option<Command> {
        let mut buf: [u8; MESSAGE_MAX_SIZE] = [0; MESSAGE_MAX_SIZE];

        let mut file = self.file.lock().unwrap();
        let len = file.read(&mut buf).unwrap() as i64;

        let mut cursor = Cursor::new(&buf[..]);
        let ret = parse(&mut cursor)
            .ok()
            .and_then(|o| Command::try_from(o).ok());

        let pos = cursor.position() as i64;
        file.seek(std::io::SeekFrom::Current(pos - len)).unwrap();

        ret
    }

    pub fn append(&self, cmd: &Command) -> Result<()> {
        match cmd {
            Command::Set(key, value) => {
                let buf = format!(
                    "*3\r\n+set\r\n+{}\r\n${}\r\n{}\r\n",
                    key,
                    value.len(),
                    value
                );
                let mut f = self.file.lock().unwrap();
                f.write_all(buf.as_bytes())?;
                f.flush()?;
                fsync(f.as_raw_fd())?;
            }
            Command::Remove(key) => {
                let buf = format!("*2\r\n+remove\r\n+{}\r\n", key);
                let mut f = self.file.lock().unwrap();
                f.write_all(buf.as_bytes())?;
                f.flush()?;
                fsync(f.as_raw_fd())?;
            }
            _ => {}
        }
        Ok(())
    }
}
