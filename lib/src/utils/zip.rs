use std::io::{Read, Seek};

use anyhow::{Context, Result};
use zip::ZipArchive;

use super::text::url_decode;

pub fn read_zip_entry_by_name<R: Read + Seek>(archive: &mut ZipArchive<R>, name: &str) -> Result<String> {
	let mut entry = archive.by_name(name).with_context(|| format!("Failed to get entry '{name}'"))?;
	let mut contents = String::new();
	entry.read_to_string(&mut contents).with_context(|| format!("Failed to read entry '{name}'"))?;
	Ok(contents)
}

pub fn find_zip_entry<R: Read + Seek>(archive: &mut ZipArchive<R>, filename: &str) -> Option<usize> {
	for i in 0..archive.len() {
		if let Ok(entry) = archive.by_index(i) {
			if entry.name() == filename {
				return Some(i);
			}
		}
	}
	let decoded = url_decode(filename);
	if decoded != filename {
		for i in 0..archive.len() {
			if let Ok(entry) = archive.by_index(i) {
				if entry.name() == decoded {
					return Some(i);
				}
			}
		}
	}
	for i in 0..archive.len() {
		if let Ok(entry) = archive.by_index(i) {
			let entry_name = entry.name();
			let decoded_entry_name = url_decode(entry_name);
			if decoded_entry_name == filename || decoded_entry_name == decoded {
				return Some(i);
			}
		}
	}
	None
}

#[cfg(test)]
mod tests {
	use std::io::{Cursor, Write};

	use zip::{ZipWriter, write::FileOptions};

	use super::*;

	#[test]
	fn test_zip_helpers() {
		let mut buffer = Cursor::new(Vec::new());
		{
			let mut zip = ZipWriter::new(&mut buffer);
			zip.start_file("hello.txt", FileOptions::default()).unwrap();
			zip.write_all(b"Hello world!").unwrap();
			zip.start_file("config%20file.ini", FileOptions::default()).unwrap();
			zip.write_all(b"[section]\nvalue=1").unwrap();
			zip.start_file("weird%20name.txt", FileOptions::default()).unwrap();
			zip.write_all(b"decoded").unwrap();
		}
		buffer.set_position(0);
		let mut archive = ZipArchive::new(buffer).unwrap();
		let text = read_zip_entry_by_name(&mut archive, "hello.txt").unwrap();
		assert_eq!(text, "Hello world!");
		let mut archive = ZipArchive::new(archive.into_inner()).unwrap();
		let index = find_zip_entry(&mut archive, "hello.txt");
		assert!(index.is_some());
		let mut archive = ZipArchive::new(archive.into_inner()).unwrap();
		let index = find_zip_entry(&mut archive, "config file.ini");
		assert!(index.is_some());
		let mut archive = ZipArchive::new(archive.into_inner()).unwrap();
		let index = find_zip_entry(&mut archive, "weird name.txt");
		assert!(index.is_some());
		let mut archive = ZipArchive::new(archive.into_inner()).unwrap();
		let index = find_zip_entry(&mut archive, "does_not_exist.txt");
		assert!(index.is_none());
	}
}
