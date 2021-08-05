use crate::command::Command;
use crate::object::parse;
use crate::server::MESSAGE_MAX_SIZE;
use nix::unistd::fsync;
use std::convert::TryFrom;
use std::fs::{File, OpenOptions};
use std::io::prelude::*;
use std::io::{Cursor, Result};
use std::os::unix::io::AsRawFd;
use std::sync::Mutex;

pub struct Wal {
    fsync: bool,
    file: Mutex<File>,
}

impl Wal {
    pub fn new(fsync: bool) -> Result<Self> {
        let path = "./wal.txt";
        let file = Mutex::new(
            OpenOptions::new()
                .create(true)
                .read(true)
                .append(true)
                .open(path)?,
        );
        Ok(Self { fsync, file })
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
        if let Some(buf) = match cmd {
            Command::Set(key, value) => Some(format!(
                "*3\r\n+set\r\n+{}\r\n${}\r\n{}\r\n",
                key,
                value.len(),
                value
            )),
            Command::Remove(key) => Some(format!("*2\r\n+remove\r\n+{}\r\n", key)),
            _ => None,
        } {
            let mut f = self.file.lock().unwrap();
            f.write_all(buf.as_bytes())?;
            if self.fsync {
                f.flush()?;
                fsync(f.as_raw_fd())?;
            }
        };
        Ok(())
    }
}
