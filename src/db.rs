use crate::command::Command;
use crate::object::Object;
use std::collections::HashMap;
use std::fmt::Display;
use std::string::FromUtf8Error;
use std::sync::{PoisonError, RwLock, RwLockReadGuard, RwLockWriteGuard};

pub trait Database: Send + Sync {
    fn execute(&self, cmd: Command) -> DbResult<Object>;
}

pub type DbResult<'a, T> = Result<T, DbError<'a>>;

#[derive(Debug)]
pub enum DbError<'a> {
    ReadLock(PoisonError<RwLockReadGuard<'a, HashMap<Object, Object>>>),
    WriteLock(PoisonError<RwLockWriteGuard<'a, HashMap<Object, Object>>>),
    Utf8(FromUtf8Error),
}

impl<'a> Display for DbError<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DbError::ReadLock(inner) => write!(f, "{}", inner),
            DbError::WriteLock(inner) => write!(f, "{}", inner),
            DbError::Utf8(inner) => write!(f, "{}", inner),
        }
    }
}

impl<'a> From<PoisonError<RwLockReadGuard<'a, HashMap<Object, Object>>>> for DbError<'a> {
    fn from(err: PoisonError<RwLockReadGuard<'a, HashMap<Object, Object>>>) -> Self {
        DbError::ReadLock(err)
    }
}

impl<'a> From<PoisonError<RwLockWriteGuard<'a, HashMap<Object, Object>>>> for DbError<'a> {
    fn from(err: PoisonError<RwLockWriteGuard<'a, HashMap<Object, Object>>>) -> Self {
        DbError::WriteLock(err)
    }
}

impl<'a> From<FromUtf8Error> for DbError<'a> {
    fn from(err: FromUtf8Error) -> Self {
        DbError::Utf8(err)
    }
}

pub struct HashMapDatabase {
    db: RwLock<HashMap<Object, Object>>,
}

impl HashMapDatabase {
    pub fn new() -> Self {
        Self {
            db: RwLock::new(HashMap::new()),
        }
    }

    fn get(&self, key: Vec<u8>) -> DbResult<Object> {
        let key = Object::SimpleString(String::from_utf8(key)?);
        let old = self
            .db
            .read()?
            .get(&key)
            .cloned()
            .unwrap_or(Object::BulkString(None));
        Ok(old)
    }

    fn set(&self, key: Vec<u8>, value: Object) -> DbResult<Object> {
        let key = Object::SimpleString(String::from_utf8(key)?);
        let old = self
            .db
            .write()?
            .insert(key, value)
            .unwrap_or(Object::BulkString(None));
        Ok(old)
    }

    fn remove(&self, key: Vec<u8>) -> DbResult<Object> {
        let key = Object::SimpleString(String::from_utf8(key)?);
        let old = self
            .db
            .write()?
            .remove(&key)
            .unwrap_or(Object::BulkString(None));
        Ok(old)
    }
}

impl Database for HashMapDatabase {
    fn execute(&self, cmd: Command) -> DbResult<Object> {
        match cmd {
            Command::Get(key) => self.get(key.into()),
            Command::Set(key, value) => self.set(key.into(), Object::BulkString(Some(value))),
            Command::Remove(key) => self.remove(key.into()),
        }
    }
}
