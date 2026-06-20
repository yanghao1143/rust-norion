use super::DiskKvStore;
use super::format::{MAGIC, OP_PUT};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
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
