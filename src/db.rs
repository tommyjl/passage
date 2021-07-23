use std::collections::HashMap;
use std::sync::RwLock;

pub trait Database: Send + Sync {
    fn get(&self, key: Vec<u8>) -> Option<Vec<u8>>;

    fn set(&self, key: Vec<u8>, value: Vec<u8>) -> Option<Vec<u8>>;

    fn remove(&self, key: Vec<u8>) -> Option<Vec<u8>>;
}

pub struct HashMapDatabase {
    db: RwLock<HashMap<Vec<u8>, Vec<u8>>>,
}

impl HashMapDatabase {
    pub fn new() -> Self {
        Self {
            db: RwLock::new(HashMap::new()),
        }
    }
}

impl Database for HashMapDatabase {
    fn get(&self, key: Vec<u8>) -> Option<Vec<u8>> {
        self.db.read().unwrap().get(&key).cloned()
    }

    fn set(&self, key: Vec<u8>, value: Vec<u8>) -> Option<Vec<u8>> {
        self.db.write().unwrap().insert(key, value)
    }

    fn remove(&self, key: Vec<u8>) -> Option<Vec<u8>> {
        self.db.write().unwrap().remove(&key)
    }
}
