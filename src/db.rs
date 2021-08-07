use crate::command::Command;
use crate::object::Object;
use std::collections::HashMap;
use std::sync::RwLock;

pub trait Database: Send + Sync {
    fn execute(&self, cmd: Command) -> Result<Object, ()>;
}

pub struct HashMapDatabase {
    db: RwLock<HashMap<Vec<u8>, Object>>,
}

impl HashMapDatabase {
    pub fn new() -> Self {
        Self {
            db: RwLock::new(HashMap::new()),
        }
    }

    fn get(&self, key: Vec<u8>) -> Result<Object, ()> {
        self.db.read().unwrap().get(&key).cloned().ok_or(())
    }

    fn set(&self, key: Vec<u8>, value: Object) -> Result<Object, ()> {
        self.db.write().unwrap().insert(key, value).ok_or(())
    }

    fn remove(&self, key: Vec<u8>) -> Result<Object, ()> {
        self.db.write().unwrap().remove(&key).ok_or(())
    }
}

impl Database for HashMapDatabase {
    fn execute(&self, cmd: Command) -> Result<Object, ()> {
        match cmd {
            Command::Get(key) => self.get(key.into()),
            Command::Set(key, value) => self.set(key.into(), Object::BulkString(Some(value))),
            Command::Remove(key) => self.remove(key.into()),
        }
    }
}
