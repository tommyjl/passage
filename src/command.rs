use crate::command_parser;
use crate::command_parser::Result;

#[derive(Debug)]
pub enum Command {
    Get(String),
    Set(String, String),
    Remove(String),
}

impl Command {
    pub fn parse(input: &[u8]) -> Result<Self> {
        command_parser::parse(input)
    }
}
