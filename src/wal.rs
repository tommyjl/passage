use crate::command::Command;
use crate::command_parser::parse;
use crate::server::MESSAGE_MAX_SIZE;
use std::fs::{File, OpenOptions};
use std::io::prelude::*;
use std::io::Result;
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
        let len = file.read(&mut buf).unwrap();

        let i = buf.iter().position(|&c| c == b'\r').unwrap();
        file.seek(std::io::SeekFrom::Current(i as i64 + 2 - len as i64))
            .unwrap();

        parse(&buf[0..len]).ok()
    }

    pub fn append(&self, cmd: &Command) -> Result<()> {
        match cmd {
            Command::Set(key, value) => {
                let mut f = self.file.lock().unwrap();
                let mut buf = String::new();
                buf.push_str("set ");
                buf.push_str(key);
                buf.push_str(" ");
                buf.push_str(value);
                buf.push_str("\r\n");
                f.write(buf.as_bytes())?;
            }
            Command::Remove(key) => {
                let mut f = self.file.lock().unwrap();
                let mut buf = String::new();
                buf.push_str("remove ");
                buf.push_str(key);
                buf.push_str("\r\n");
                f.write(buf.as_bytes())?;
            }
            _ => {}
        }
        Ok(())
    }
}
