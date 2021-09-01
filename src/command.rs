use crate::object::Object;
use std::convert::TryFrom;

#[derive(Debug)]
pub enum Command {
    Get(String),
    Set(String, String),
    Remove(String),
}

impl Command {
    pub fn possibly_dirty(&self) -> bool {
        match self {
            Command::Get(_) => false,
            Command::Set(_, _) => true,
            Command::Remove(_) => true,
        }
    }
}

impl TryFrom<Object> for Command {
    type Error = String;

    fn try_from(obj: Object) -> Result<Self, Self::Error> {
        if let Object::Array(vec) = obj {
            Command::try_from(vec)
        } else {
            Err("Object is not a valid Command".to_string())
        }
    }
}

impl TryFrom<Vec<Object>> for Command {
    type Error = String;

    fn try_from(vec: Vec<Object>) -> Result<Self, Self::Error> {
        if vec.is_empty() {
            return Err("Empty array object".to_string());
        }

        let arity = vec.len() - 1;
        match vec[0] {
            Object::SimpleString(ref s) => match (s.as_str(), arity) {
                ("get", 1) => Ok(Command::Get(get_string(&vec[1])?)),
                ("set", 2) => Ok(Command::Set(get_string(&vec[1])?, get_string(&vec[2])?)),
                ("remove", 1) => Ok(Command::Remove(get_string(&vec[1])?)),
                _ => Err("Unknown command".to_string()),
            },
            _ => Err("Unknown command".to_string()),
        }
    }
}

fn get_string(obj: &Object) -> Result<String, String> {
    match obj {
        Object::SimpleString(ref s) => Ok(s.clone()),
        Object::BulkString(Some(ref s)) => Ok(s.clone()),
        _ => Err("Unsupported type".to_string()),
    }
}
