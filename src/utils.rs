use std::sync::RwLock;
use chrono::TimeZone;

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

pub fn local_datetime_from_millis(millis: i64) -> chrono::DateTime<chrono::Local> {
    match chrono::Local.timestamp_millis_opt(millis) {
        chrono::offset::LocalResult::Single(ts) => ts,
        chrono::offset::LocalResult::Ambiguous(ts, _) => ts,
        _ => chrono::Local::now(),
    }
}