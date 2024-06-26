use crate::error::FilesystemError;
use crate::fs::backup::{Backup, ZipBackup, ZstdBackup};

use chrono::{Local, NaiveDateTime};
use std::convert::TryFrom;
use std::path::{Path, PathBuf};

use std::fmt::Display;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Hash, PartialOrd, Ord, Deserialize)]
pub enum CompressionFormat {
	Zip,
	Zstd,
}

impl CompressionFormat {
	pub const ALL: [CompressionFormat; 2] = [CompressionFormat::Zip, CompressionFormat::Zstd];

	pub(crate) const fn file_ext(&self) -> &'static str {
		match self {
			CompressionFormat::Zip => "zip",
			CompressionFormat::Zstd => "tar.zst",
		}
	}
}

impl Default for CompressionFormat {
	fn default() -> CompressionFormat {
		CompressionFormat::Zip
	}
}

impl Display for CompressionFormat {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			CompressionFormat::Zip => f.write_str("Zip"),
			CompressionFormat::Zstd => f.write_str("Zstd"),
		}
	}
}

impl FromStr for CompressionFormat {
	type Err = &'static str;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		match s {
			"zip" | "Zip" => Ok(CompressionFormat::Zip),
			"zstd" | "Zstd" => Ok(CompressionFormat::Zstd),
			_ => Err("valid values are: zip, zstd"),
		}
	}
}

/// Creates a .zip archive from the list of source folders and
/// saves it to the dest folder.
pub async fn backup_folders(
	src_folders: Vec<BackupFolder>,
	mut dest: PathBuf,
	compression: CompressionFormat,
	zstd_level: i32,
) -> Result<NaiveDateTime, FilesystemError> {
	let now = Local::now();

	dest.push(format!(
		"grin_gui_backup_{}.{}",
		now.format("%Y-%m-%d_%H-%M-%S"),
		compression.file_ext(),
	));

	match compression {
		CompressionFormat::Zip => ZipBackup::new(src_folders, &dest).backup()?,
		CompressionFormat::Zstd => ZstdBackup::new(src_folders, &dest, zstd_level).backup()?,
	}

	// Won't fail since we pass it the correct format
	let as_of = Archive::try_from(dest).unwrap().as_of;

	Ok(as_of)
}

/// Finds the latest archive in the supplied backup folder and returns
/// the datetime it was saved
pub async fn latest_backup(backup_dir: PathBuf) -> Option<NaiveDateTime> {
	let zip_pattern = format!("{}/grin_gui_backup_[0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]_[0-9][0-9]-[0-9][0-9]-[0-9][0-9].zip", backup_dir.display());
	let zstd_pattern = format!("{}/grin_gui_backup_[0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]_[0-9][0-9]-[0-9][0-9]-[0-9][0-9].tar.zst", backup_dir.display());

	let mut backups = vec![];

	for path in glob::glob(&zip_pattern)
		.unwrap()
		.chain(glob::glob(&zstd_pattern).unwrap())
		.flatten()
	{
		if let Ok(archive) = Archive::try_from(path) {
			backups.push(archive.as_of);
		}
	}

	// Apparently NaiveDateTime sorts in Desc order by default, no need to reverse
	backups.sort();
	backups.pop()
}

/// Specifies a folder that we want backed up. `prefix` will get stripped out of
/// the path of each entry in the archive.
pub struct BackupFolder {
	pub path: PathBuf,
	pub prefix: PathBuf,
}

impl BackupFolder {
	pub fn new(path: impl AsRef<Path>, prefix: impl AsRef<Path>) -> BackupFolder {
		BackupFolder {
			path: path.as_ref().to_owned(),
			prefix: prefix.as_ref().to_owned(),
		}
	}
}

/// Metadata for our archive saved on the filesystem. Converted from a `PathBuf` with
/// the correct naming convention
struct Archive {
	pub as_of: NaiveDateTime,
}

impl TryFrom<PathBuf> for Archive {
	type Error = chrono::ParseError;

	fn try_from(path: PathBuf) -> Result<Archive, chrono::ParseError> {
		let mut file_stem = path.file_stem().unwrap().to_str().unwrap();

		// in the case of "file.tar.zst" path.file_stem() will return "file.tar", we still need to
		// drop the extension
		if let Some(i) = file_stem.find('.') {
			file_stem = file_stem.split_at(i).0;
		}

		let date_str = format!(
			"{} {}",
			file_stem.split('_').nth(2).unwrap_or_default(),
			file_stem.split('_').nth(3).unwrap_or_default()
		);

		let as_of = NaiveDateTime::parse_from_str(&date_str, "%Y-%m-%d %H-%M-%S")?;

		Ok(Archive { as_of })
	}
}
