use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{self, ErrorKind, Seek, SeekFrom};
use std::path::Path;

use super::format::{OP_DELETE, OP_PUT, read_record};

#[derive(Debug, Clone)]
pub(super) struct RecordPointer {
    pub(super) value_offset: u64,
    pub(super) value_len: u64,
}

pub(super) fn scan_index(path: &Path) -> io::Result<HashMap<String, RecordPointer>> {
    let mut file = OpenOptions::new().read(true).write(true).open(path)?;
    scan_index_from_file(&mut file, true)
}

pub(super) fn scan_index_read_only(path: &Path) -> io::Result<HashMap<String, RecordPointer>> {
    let mut file = OpenOptions::new().read(true).open(path)?;
    scan_index_from_file(&mut file, false)
}

fn scan_index_from_file(
    file: &mut File,
    repair_partial_tail: bool,
) -> io::Result<HashMap<String, RecordPointer>> {
    let mut index = HashMap::new();
    let mut offset = 0;

    loop {
        file.seek(SeekFrom::Start(offset))?;
        let Some(record) = read_record(file, offset)? else {
            if repair_partial_tail {
                file.set_len(offset)?;
            }
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
