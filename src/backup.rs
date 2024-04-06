use crate::*;

use trash::delete;
use std::{fs::File, path::PathBuf, sync::{Mutex, RwLock}};
use serde::{Serialize, Deserialize};

#[derive(Clone)]
pub enum BackupState {
    Idle,
    Busy,
    Finished,
}

pub static BACKUP_STATE: RwLock<BackupState> = RwLock::new(BackupState::Idle);
pub static BACKUP_ERROR: RwLock<String> = RwLock::new(String::new());
pub static BACKUP_NAME: RwLock<String> = RwLock::new(String::new());

pub static BACKUP_PATH: Mutex<String> = Mutex::new(String::new());
pub static BACKUP_LIST: Mutex<Vec<SavegameMeta>> = Mutex::new(vec![]);


#[derive(Clone, Default, Serialize, Deserialize)]
pub struct SavegameMeta {
    #[serde(skip)] pub name: String,
    pub date: i64,
    pub checksums: Vec<(String, String)>,
}

impl SavegameMeta {
    pub fn is_temp(&self) -> bool {
        self.name.starts_with("temp_")
    }
    pub fn is_auto(&self) -> bool {
        self.name.starts_with("auto_")
    }
}

fn take_backup(src_path: &String, dst_path: &String, backup_name: &String, copy_screenshot: &bool) -> Result<(), anyhow::Error> {
    let src_pathbuf = PathBuf::from(src_path);
    let dst_pathbuf = PathBuf::from(dst_path).join(backup_name);

    std::fs::create_dir(&dst_pathbuf)?;

    let mut file_list: Vec<PathBuf> = vec![];
    let mut checksum_list: Vec<String> = vec![];
    for entry in std::fs::read_dir(&src_pathbuf)? {
        let entry_path = entry?.path();
        if entry_path.is_file() {
            checksum_list.push(fhc::file_blake3(&entry_path)?);
            file_list.push(entry_path);
        }
    }

    let mut meta_checksums: Vec<(String, String)> = vec![];
    for (i, file) in file_list.iter().enumerate() {
        let file_name = String::from(file.file_name().unwrap_or_default().to_str().unwrap_or_default());
        let checksum = checksum_list[i].clone();

        let new_path = dst_pathbuf.join(&file_name);
        std::fs::copy(&file, &new_path)?;

        meta_checksums.push((file_name, checksum));
    }

    if *copy_screenshot {
        let screenshot_path = PathBuf::from("screenshot.jpg");
        if screenshot_path.exists() && screenshot_path.is_file() {
            std::fs::copy("screenshot.jpg", dst_pathbuf.join("screenshot.jpg")).unwrap_or_default();
            std::fs::remove_file(screenshot_path).unwrap_or_default();
        }
    }

    let meta_file = File::create(dst_pathbuf.join("meta.json"))?;
    let now = chrono::Local::now();
    serde_json::to_writer_pretty(meta_file, &SavegameMeta { name: backup_name.clone(), date: now.timestamp_millis(), checksums: meta_checksums })?;

    Ok(())
}


pub fn create_backup(src_path: &String, dst_path: &String, backup_name: &String, copy_screenshot: &bool) {
    if src_path.is_empty() || dst_path.is_empty() {
        println!("Source or destination path is empty");
        write_to_rwlock(&BACKUP_ERROR, String::new());
        write_to_rwlock(&BACKUP_NAME, String::new());
        write_to_rwlock(&BACKUP_STATE, BackupState::Finished);
        return;
    }

    match take_backup(src_path, dst_path, &backup_name, copy_screenshot) {
        Ok(_) => {
            write_to_rwlock(&BACKUP_ERROR, String::new());
            write_to_rwlock(&BACKUP_NAME, backup_name.clone());
            write_to_rwlock(&BACKUP_STATE, BackupState::Finished);
        },
        Err(err) => {
            println!("Error creating backup from {} to {}: {:?}", src_path, dst_path, err);
            write_to_rwlock(&BACKUP_ERROR, format!("Error creating backup: {}", err));
            write_to_rwlock(&BACKUP_NAME, String::new());
            write_to_rwlock(&BACKUP_STATE, BackupState::Finished);
        }
    }
}

pub fn create_autosave(src_path: &String, dst_path: &String, copy_screenshot: &bool, max_autosaves: &u16) {
    let _ = look_for_backups(dst_path);

    let mut backup_list = BACKUP_LIST.lock().unwrap();
    backup_list.sort_by(|a, b| b.date.cmp(&a.date));
    let mut auto_count = 0;
    for backup in &*backup_list {
        if backup.is_auto() {
            auto_count += 1;
            if auto_count >= *max_autosaves {
                let _ = delete_backup(dst_path, &backup.name);
            }
        } else if backup.is_temp() {
            let _ = delete_backup(dst_path, &backup.name);
        }
    }
    drop(backup_list);

    let now = chrono::Local::now();
    let backup_name = now.format("auto_%Y-%m-%d_%H-%M-%S").to_string();
    create_backup(src_path, dst_path, &backup_name, copy_screenshot);
}

pub fn create_tempsave(src_path: &String, dst_path: &String, copy_screenshot: &bool) {
    let _ = look_for_backups(dst_path);

    let mut backup_list = BACKUP_LIST.lock().unwrap();
    backup_list.sort_by(|a, b| b.date.cmp(&a.date));
    for backup in &*backup_list {
        if backup.is_temp() {
            let _ = delete_backup(dst_path, &backup.name);
        }
    }
    drop(backup_list);

    let now = chrono::Local::now();
    let backup_name = now.format("temp_%Y-%m-%d_%H-%M-%S").to_string();

    create_backup(src_path, dst_path, &backup_name, copy_screenshot);
}

pub fn create_savetokeep(src_path: &String, dst_path: &String, copy_screenshot: &bool) {
    let _ = look_for_backups(dst_path);

    let mut backup_list = BACKUP_LIST.lock().unwrap();
    backup_list.sort_by(|a, b| b.date.cmp(&a.date));
    for backup in &*backup_list {
        if backup.is_temp() {
            let _ = delete_backup(dst_path, &backup.name);
        }
    }
    drop(backup_list);

    let now = chrono::Local::now();
    let backup_name = now.format("%Y-%m-%d_%H-%M-%S").to_string();
    create_backup(src_path, dst_path, &backup_name, copy_screenshot);
}

pub fn get_meta_for_backup(dst_path: &String, backup_name: &String) -> Result<SavegameMeta, anyhow::Error> {
    let mut last_backup_path = BACKUP_PATH.lock().unwrap();
    if *last_backup_path != *dst_path {
        *last_backup_path = dst_path.clone();
        BACKUP_LIST.lock().unwrap().clear();
    }

    for backup in &*BACKUP_LIST.lock().unwrap() {
        if backup.name == *backup_name {
            return Ok(backup.clone());
        }
    }
    

    let bak_pathbuf = PathBuf::from(dst_path).join(backup_name);
    
    if bak_pathbuf.exists() && bak_pathbuf.is_dir() {
        let meta_file_path = bak_pathbuf.join("meta.json");
        if meta_file_path.exists() && meta_file_path.is_file() {
            let meta_file = File::open(meta_file_path)?;
            let mut meta: SavegameMeta = serde_json::from_reader(meta_file)?;
            meta.name = backup_name.clone();
            BACKUP_LIST.lock().unwrap().push(meta.clone());
            Ok(meta)
        } else {
            Err(anyhow::anyhow!("Backup meta file does not exist"))
        }
    } else {
        Err(anyhow::anyhow!("Backup directory does not exist or is a file"))
    }
}

pub fn look_for_backups(dst_path: &String) -> Result<Vec<SavegameMeta>, anyhow::Error> {
    let mut backups: Vec<SavegameMeta> = vec![];

    let dst_pathbuf = PathBuf::from(dst_path);
    if !dst_pathbuf.exists() || !dst_pathbuf.is_dir() {
        return Err(anyhow::anyhow!("Destination path does not exist or is not a directory"));
    }

    let files = std::fs::read_dir(dst_pathbuf)?;
    for file in files {
        let backup_name = String::from(file?.path().file_name().unwrap_or_default().to_str().unwrap_or_default());

        match get_meta_for_backup(dst_path, &backup_name) {
            Ok(meta) => backups.push(meta),
            Err(err) => {
                if PathBuf::from(dst_path).join(&backup_name).is_dir() {
                    println!("Error reading backup meta for {}: {:?}", backup_name, err)
                }
            },
        }
    }

    Ok(backups)
}

pub fn create_hash_list(path: &String) -> Vec<(String, String)> {
    let pathbuf = PathBuf::from(path);
    let mut hash_list: Vec<(String, String)> = vec![];

    if pathbuf.exists() && pathbuf.is_dir() {
        for entry in std::fs::read_dir(&pathbuf).unwrap() {
            let entry_path = entry.unwrap().path();
            if entry_path.is_file() {
                hash_list.push((String::from(entry_path.file_name().unwrap_or_default().to_str().unwrap_or_default()), fhc::file_blake3(&entry_path).unwrap()));
            }
        }
    }

    hash_list
}

pub enum BackupComparison {
    CompleteDiff,
    PartialDiff,
    NoDiff,
}

impl std::ops::Not for BackupComparison {
    type Output = bool;

    fn not(self) -> Self::Output {
        match self {
            BackupComparison::CompleteDiff => true,
            BackupComparison::PartialDiff => true,
            BackupComparison::NoDiff => false,
        }
    }
}

pub fn hash_list_cmp(hashes: &Vec<(String, String)>, cmp_with: &Vec<(String, String)>) -> BackupComparison {
    let mut some_matches = false;
    let mut all_matches = true;

    for hash in hashes {
        let mut file_found = false;
        for cmp_hash in cmp_with {
            if hash.0 == cmp_hash.0 {
                file_found = true;
                if hash.1 != cmp_hash.1 {
                    all_matches = false;
                } else {
                    some_matches = true;
                }
                break;
            }
        }

        if !file_found {
            all_matches = false;
        }
    }

    if all_matches {
        BackupComparison::NoDiff
    } else if some_matches {
        BackupComparison::PartialDiff
    } else {
        BackupComparison::CompleteDiff
    }
}

pub fn load_backup(src_path: &String, dst_path: &String, backup: &SavegameMeta) -> Result<(), anyhow::Error> {
    if !read_rwlock_or(&crate::WATCHER_PAUSED, false) {
        return Err(anyhow::anyhow!("Cannot load backup while watcher is running"));
    }

    let hash_list = create_hash_list(src_path);
    if !hash_list_cmp(&backup.checksums, &hash_list) {
        for (file, _) in &hash_list {
            std::fs::remove_file(PathBuf::from(src_path).join(file))?;
        }

        for (file, _) in &backup.checksums {
            std::fs::copy(PathBuf::from(dst_path).join(&backup.name).join(file), PathBuf::from(src_path).join(file))?;
        }
    }

    Ok(())
}

pub fn deal_with_exit_save(dst_path: &String) {
    let _ = look_for_backups(dst_path);

    let mut backup_list = BACKUP_LIST.lock().unwrap();
    backup_list.sort_by(|a, b| b.date.cmp(&a.date));
    let mut first_temp = true;
    for backup in &*backup_list {
        if backup.is_temp() {
            if first_temp {
                let _ = rename_backup(dst_path, &backup.name, &backup.name.replace("temp_", "exit_"));
                first_temp = false;
            } else {
                let _ = delete_backup(dst_path, &backup.name);
            }
        }
    }
    drop(backup_list);
}

pub fn rename_backup(dst_path: &String, old_name: &String, new_name: &String) -> std::io::Result<()> {
    let old_path = PathBuf::from(dst_path).join(old_name);
    let new_path = PathBuf::from(dst_path).join(new_name);

    if old_path.exists() && old_path.is_dir() && !new_path.exists() {
        std::fs::rename(old_path, new_path)
    } else {
        Ok(())
    }
}

pub fn recycle_backup(dst_path: &String, backup_name: &String) -> Result<(), trash::Error> {
    let backup_path = PathBuf::from(dst_path).join(backup_name);

    if backup_path.exists() && backup_path.is_dir() {
        delete(backup_path)
    } else {
        Ok(())
    }
}

pub fn delete_backup(dst_path: &String, backup_name: &String) -> Result<(), std::io::Error> {
    let backup_path = PathBuf::from(dst_path).join(backup_name);
    if backup_path.exists() && backup_path.is_dir() {
        std::fs::remove_dir_all(backup_path)
    } else {
        Ok(())
    }
}