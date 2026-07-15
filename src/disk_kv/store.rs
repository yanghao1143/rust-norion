use std::collections::HashMap;
use std::fs::{self, OpenOptions};
use std::io::{self, ErrorKind, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use super::format::{HEADER_LEN, OP_DELETE, OP_PUT, validate_key_value, write_record};
use super::index::{RecordPointer, scan_index, scan_index_read_only};

#[derive(Debug, Clone)]
pub struct DiskKvStore {
    path: PathBuf,
    index: HashMap<String, RecordPointer>,
    read_only: bool,
}

impl DiskKvStore {
    pub fn open(path: impl AsRef<Path>) -> io::Result<Self> {
        let path = path.as_ref().to_path_buf();
        if let Some(parent) = path.parent()
            && !parent.as_os_str().is_empty()
        {
            fs::create_dir_all(parent)?;
        }
        restore_compaction_backup_if_needed(&path)?;
        OpenOptions::new()
            .create(true)
            .append(true)
            .read(true)
            .open(&path)?;

        let index = scan_index(&path)?;
        Ok(Self {
            path,
            index,
            read_only: false,
        })
    }

    pub fn open_read_only_existing(path: impl AsRef<Path>) -> io::Result<Option<Self>> {
        let path = path.as_ref().to_path_buf();
        let index = match scan_index_read_only(&path) {
            Ok(index) => index,
            Err(error) if error.kind() == ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error),
        };
        Ok(Some(Self {
            path,
            index,
            read_only: true,
        }))
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn len(&self) -> usize {
        self.index.len()
    }

    pub fn is_empty(&self) -> bool {
        self.index.is_empty()
    }

    pub fn contains_key(&self, key: &str) -> bool {
        self.index.contains_key(key)
    }

    pub fn keys(&self) -> Vec<String> {
        let mut keys = self.index.keys().cloned().collect::<Vec<_>>();
        keys.sort();
        keys
    }

    pub fn keys_with_prefix(&self, prefix: &str) -> Vec<String> {
        let mut keys = self
            .index
            .keys()
            .filter(|key| key.starts_with(prefix))
            .cloned()
            .collect::<Vec<_>>();
        keys.sort();
        keys
    }

    pub fn put(&mut self, key: impl AsRef<str>, value: impl AsRef<[u8]>) -> io::Result<()> {
        self.ensure_writable()?;
        let key = key.as_ref();
        let value = value.as_ref();
        validate_key_value(key.as_bytes(), value)?;

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .read(true)
            .open(&self.path)?;
        let offset = file.seek(SeekFrom::End(0))?;
        write_record(&mut file, OP_PUT, key.as_bytes(), value)?;
        file.sync_data()?;

        self.index.insert(
            key.to_owned(),
            RecordPointer {
                value_offset: offset + HEADER_LEN + key.len() as u64,
                value_len: value.len() as u64,
            },
        );
        Ok(())
    }

    pub fn get(&self, key: &str) -> io::Result<Option<Vec<u8>>> {
        let Some(pointer) = self.index.get(key) else {
            return Ok(None);
        };

        let mut file = OpenOptions::new().read(true).open(&self.path)?;
        file.seek(SeekFrom::Start(pointer.value_offset))?;
        let mut value = vec![0; pointer.value_len as usize];
        file.read_exact(&mut value)?;
        Ok(Some(value))
    }

    pub fn delete(&mut self, key: &str) -> io::Result<bool> {
        self.ensure_writable()?;
        if !self.index.contains_key(key) {
            return Ok(false);
        }

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .read(true)
            .open(&self.path)?;
        write_record(&mut file, OP_DELETE, key.as_bytes(), &[])?;
        file.sync_data()?;
        self.index.remove(key);
        Ok(true)
    }

    pub fn compact(&mut self) -> io::Result<()> {
        self.ensure_writable()?;
        let mut entries = Vec::new();
        for key in self.keys() {
            if let Some(value) = self.get(&key)? {
                entries.push((key, value));
            }
        }
        let compact_path = self.path.with_extension("compact");

        {
            let mut file = OpenOptions::new()
                .create(true)
                .truncate(true)
                .write(true)
                .open(&compact_path)?;

            for (key, value) in &entries {
                write_record(&mut file, OP_PUT, key.as_bytes(), value)?;
            }
            file.sync_all()?;
        }

        let backup_path = self.path.with_extension("compact.bak");
        remove_file_if_exists(&backup_path)?;
        fs::rename(&self.path, &backup_path)?;
        if let Err(error) = fs::rename(&compact_path, &self.path) {
            let _ = fs::rename(&backup_path, &self.path);
            return Err(error);
        }
        remove_file_if_exists(&backup_path)?;
        self.index = scan_index(&self.path)?;
        Ok(())
    }

    fn ensure_writable(&self) -> io::Result<()> {
        if self.read_only {
            return Err(io::Error::new(
                ErrorKind::PermissionDenied,
                "disk kv store was opened read-only",
            ));
        }
        Ok(())
    }
}

fn restore_compaction_backup_if_needed(path: &Path) -> io::Result<()> {
    match fs::metadata(path) {
        Ok(_) => return Ok(()),
        Err(error) if error.kind() == ErrorKind::NotFound => {}
        Err(error) => return Err(error),
    }

    let backup_path = path.with_extension("compact.bak");
    match fs::rename(backup_path, path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

fn remove_file_if_exists(path: &Path) -> io::Result<()> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}
