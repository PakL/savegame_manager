use std::sync::RwLock;

pub fn read_rwlock_or<T: Clone>(lock: &RwLock<T>, default: T) -> T {
    match lock.read() {
        Ok(value) => value.clone(),
        Err(_) => default,
    }
}

pub fn write_to_rwlock<T>(lock: &RwLock<T>, value: T) {
    match lock.write() {
        Ok(mut lock) => *lock = value,
        Err(err) => println!("Error writing to RwLock: {:?}", err),
    }
}