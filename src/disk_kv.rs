use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::io::{self, ErrorKind, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

const MAGIC: &[u8; 4] = b"NDK1";
const OP_PUT: u8 = 1;
const OP_DELETE: u8 = 2;
const HEADER_LEN: u64 = 4 + 1 + 4 + 8 + 8;
const MAX_KEY_LEN: u32 = 64 * 1024;
const MAX_VALUE_LEN: u64 = 64 * 1024 * 1024;

#[derive(Debug, Clone)]
struct RecordPointer {
    value_offset: u64,
    value_len: u64,
}

#[derive(Debug, Clone)]
pub struct DiskKvStore {
    path: PathBuf,
    index: HashMap<String, RecordPointer>,
}

impl DiskKvStore {
    pub fn open(path: impl AsRef<Path>) -> io::Result<Self> {
        let path = path.as_ref().to_path_buf();
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)?;
            }
        }
        OpenOptions::new()
            .create(true)
            .append(true)
            .read(true)
            .open(&path)?;

        let index = scan_index(&path)?;
        Ok(Self { path, index })
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

        if self.path.exists() {
            fs::remove_file(&self.path)?;
        }
        fs::rename(&compact_path, &self.path)?;
        self.index = scan_index(&self.path)?;
        Ok(())
    }
}

fn scan_index(path: &Path) -> io::Result<HashMap<String, RecordPointer>> {
    let mut file = OpenOptions::new().read(true).write(true).open(path)?;
    let mut index = HashMap::new();
    let mut offset = 0;

    loop {
        file.seek(SeekFrom::Start(offset))?;
        let Some(record) = read_record(&mut file, offset)? else {
            file.set_len(offset)?;
            break;
        };

        match record.op {
            OP_PUT => {
                index.insert(
                    record.key,
                    RecordPointer {
                        value_offset: record.value_offset,
                        value_len: record.value_len,
                    },
                );
            }
            OP_DELETE => {
                index.remove(&record.key);
            }
            _ => {
                return Err(io::Error::new(
                    ErrorKind::InvalidData,
                    format!("unknown disk kv op {}", record.op),
                ));
            }
        }

        offset = record.next_offset;
    }

    Ok(index)
}

#[derive(Debug)]
struct ScannedRecord {
    op: u8,
    key: String,
    value_offset: u64,
    value_len: u64,
    next_offset: u64,
}

fn read_record(file: &mut File, offset: u64) -> io::Result<Option<ScannedRecord>> {
    let mut magic = [0; 4];
    match file.read_exact(&mut magic) {
        Ok(()) => {}
        Err(error) if error.kind() == ErrorKind::UnexpectedEof => return Ok(None),
        Err(error) => return Err(error),
    }
    if &magic != MAGIC {
        return Err(io::Error::new(
            ErrorKind::InvalidData,
            format!("invalid disk kv magic at offset {offset}"),
        ));
    }

    let mut op = [0; 1];
    let mut key_len = [0; 4];
    let mut value_len = [0; 8];
    let mut checksum = [0; 8];
    if let Err(error) = read_header_tail(file, &mut op, &mut key_len, &mut value_len, &mut checksum)
    {
        return if error.kind() == ErrorKind::UnexpectedEof {
            Ok(None)
        } else {
            Err(error)
        };
    }

    let op = op[0];
    let key_len = u32::from_le_bytes(key_len);
    let value_len = u64::from_le_bytes(value_len);
    let expected_checksum = u64::from_le_bytes(checksum);
    validate_lengths(key_len, value_len)?;

    let mut key = vec![0; key_len as usize];
    if let Err(error) = file.read_exact(&mut key) {
        return if error.kind() == ErrorKind::UnexpectedEof {
            Ok(None)
        } else {
            Err(error)
        };
    }
    let value_offset = file.stream_position()?;
    let mut value = vec![0; value_len as usize];
    if let Err(error) = file.read_exact(&mut value) {
        return if error.kind() == ErrorKind::UnexpectedEof {
            Ok(None)
        } else {
            Err(error)
        };
    }

    let actual_checksum = checksum_record(op, &key, &value);
    if actual_checksum != expected_checksum {
        return Err(io::Error::new(
            ErrorKind::InvalidData,
            format!("disk kv checksum mismatch at offset {offset}"),
        ));
    }

    let key = String::from_utf8(key).map_err(|error| {
        io::Error::new(
            ErrorKind::InvalidData,
            format!("disk kv key is not utf-8: {error}"),
        )
    })?;
    let next_offset = file.stream_position()?;

    Ok(Some(ScannedRecord {
        op,
        key,
        value_offset,
        value_len,
        next_offset,
    }))
}

fn read_header_tail(
    file: &mut File,
    op: &mut [u8; 1],
    key_len: &mut [u8; 4],
    value_len: &mut [u8; 8],
    checksum: &mut [u8; 8],
) -> io::Result<()> {
    file.read_exact(op)?;
    file.read_exact(key_len)?;
    file.read_exact(value_len)?;
    file.read_exact(checksum)?;
    Ok(())
}

fn write_record(file: &mut File, op: u8, key: &[u8], value: &[u8]) -> io::Result<()> {
    validate_key_value(key, value)?;
    file.write_all(MAGIC)?;
    file.write_all(&[op])?;
    file.write_all(&(key.len() as u32).to_le_bytes())?;
    file.write_all(&(value.len() as u64).to_le_bytes())?;
    file.write_all(&checksum_record(op, key, value).to_le_bytes())?;
    file.write_all(key)?;
    file.write_all(value)?;
    Ok(())
}

fn validate_key_value(key: &[u8], value: &[u8]) -> io::Result<()> {
    if key.is_empty() {
        return Err(io::Error::new(
            ErrorKind::InvalidInput,
            "key cannot be empty",
        ));
    }
    validate_lengths(key.len() as u32, value.len() as u64)
}

fn validate_lengths(key_len: u32, value_len: u64) -> io::Result<()> {
    if key_len == 0 || key_len > MAX_KEY_LEN {
        return Err(io::Error::new(
            ErrorKind::InvalidData,
            format!("invalid key length {key_len}"),
        ));
    }
    if value_len > MAX_VALUE_LEN {
        return Err(io::Error::new(
            ErrorKind::InvalidData,
            format!("invalid value length {value_len}"),
        ));
    }
    Ok(())
}

fn checksum_record(op: u8, key: &[u8], value: &[u8]) -> u64 {
    let mut hash = 0xcbf29ce484222325_u64;
    hash = fnv_mix(hash, op);

    for byte in key.iter().chain(value.iter()) {
        hash = fnv_mix(hash, *byte);
    }

    hash
}

fn fnv_mix(mut hash: u64, byte: u8) -> u64 {
    hash ^= u64::from(byte);
    hash.wrapping_mul(0x100000001b3)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn put_get_and_reopen() {
        let path = temp_path("put-get");
        let mut store = DiskKvStore::open(&path).unwrap();
        store.put("memory/1", b"hello").unwrap();
        store.put("memory/2", b"world").unwrap();

        assert_eq!(store.get("memory/1").unwrap().unwrap(), b"hello");
        assert_eq!(store.len(), 2);

        let reopened = DiskKvStore::open(&path).unwrap();
        assert_eq!(reopened.get("memory/2").unwrap().unwrap(), b"world");
        cleanup(path);
    }

    #[test]
    fn delete_is_persistent() {
        let path = temp_path("delete");
        let mut store = DiskKvStore::open(&path).unwrap();
        store.put("memory/1", b"hello").unwrap();
        assert!(store.delete("memory/1").unwrap());

        let reopened = DiskKvStore::open(&path).unwrap();
        assert!(!reopened.contains_key("memory/1"));
        cleanup(path);
    }

    #[test]
    fn compact_keeps_latest_values() {
        let path = temp_path("compact");
        let mut store = DiskKvStore::open(&path).unwrap();
        store
            .put("memory/1", b"old value that should disappear")
            .unwrap();
        store.put("memory/1", b"new").unwrap();
        store.put("memory/2", b"stable").unwrap();
        let before = fs::metadata(&path).unwrap().len();

        store.compact().unwrap();
        let after = fs::metadata(&path).unwrap().len();

        assert!(after < before);
        assert_eq!(store.get("memory/1").unwrap().unwrap(), b"new");
        assert_eq!(store.get("memory/2").unwrap().unwrap(), b"stable");
        cleanup(path);
    }

    #[test]
    fn open_truncates_partial_tail_record() {
        let path = temp_path("partial-tail");
        let mut store = DiskKvStore::open(&path).unwrap();
        store.put("memory/1", b"stable").unwrap();
        let clean_len = fs::metadata(&path).unwrap().len();

        {
            let mut file = OpenOptions::new().append(true).open(&path).unwrap();
            file.write_all(MAGIC).unwrap();
            file.write_all(&[OP_PUT]).unwrap();
        }

        let reopened = DiskKvStore::open(&path).unwrap();

        assert_eq!(reopened.get("memory/1").unwrap().unwrap(), b"stable");
        assert_eq!(fs::metadata(&path).unwrap().len(), clean_len);
        cleanup(path);
    }

    fn temp_path(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "rust-norion-{label}-{}-{nanos}.ndkv",
            std::process::id()
        ))
    }

    fn cleanup(path: PathBuf) {
        let _ = fs::remove_file(path);
    }
}
