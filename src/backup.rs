use crate::*;

use std::{fs::File, path::PathBuf, sync::RwLock};
use serde::{Serialize, Deserialize};

pub static BACKUP_STATE: RwLock<u8> = RwLock::new(0);
pub static BACKUP_ERROR: RwLock<String> = RwLock::new(String::new());
pub static BACKUP_NAME: RwLock<String> = RwLock::new(String::new());


#[derive(Clone, Default, Serialize, Deserialize)]
pub struct SavegameMeta {
    pub name: String,
    pub date: i64,
    pub checksums: Vec<(String, String)>,
}

fn take_backup(src_path: &String, dst_path: &String, copy_screenshot: &bool) -> Result<String, anyhow::Error> {
	let now = chrono::Local::now();
	let backup_name = now.format("%Y%m%d%H%M%S").to_string();

	let src_pathbuf = PathBuf::from(src_path);
	let dst_pathbuf = PathBuf::from(dst_path).join(&backup_name);

	std::fs::create_dir(&dst_pathbuf)?;

	let mut file_list: Vec<PathBuf> = vec![];
	let mut checksum_list: Vec<String> = vec![];
	for entry in std::fs::read_dir(&src_pathbuf)? {
		let entry_path = entry?.path();
		if entry_path.is_file() {
			checksum_list.push(fhc::file_sha256(&entry_path)?);
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
		std::fs::copy("screenshot.jpg", dst_pathbuf.join("screenshot.jpg"))?;
	}

	let meta_file = File::create(dst_pathbuf.join("meta.json"))?;
	serde_json::to_writer_pretty(meta_file, &SavegameMeta { name: backup_name.clone(), date: now.timestamp_millis(), checksums: meta_checksums })?;

	Ok(backup_name)
}

pub fn create_backup(src_path: &String, dst_path: &String, copy_screenshot: &bool) {
	match take_backup(src_path, dst_path, copy_screenshot) {
		Ok(backup_name) => {
			write_to_rwlock(&BACKUP_ERROR, String::new());
			write_to_rwlock(&BACKUP_NAME, backup_name);
			write_to_rwlock(&BACKUP_STATE, 2);
		},
		Err(err) => {
			println!("Error creating backup from {} to {}: {:?}", src_path, dst_path, err);
			write_to_rwlock(&BACKUP_ERROR, format!("Error creating backup: {}", err));
			write_to_rwlock(&BACKUP_NAME, String::new());
			write_to_rwlock(&BACKUP_STATE, 2);
		}
	}
}

pub fn get_meta_for_backup(dst_path: &String, backup_name: &String) -> Result<SavegameMeta, anyhow::Error> {
	let bak_pathbuf = PathBuf::from(dst_path).join(backup_name);
	
	if bak_pathbuf.is_dir() {
		let meta_file_path = bak_pathbuf.join("meta.json");
		if meta_file_path.exists() {
			let meta_file = File::open(meta_file_path)?;
			Ok(serde_json::from_reader(meta_file)?)
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
			Err(err) => println!("Error reading backup meta for {}: {:?}", backup_name, err),
		}
	}

	Ok(backups)
}