use std::fs::File;
use std::io::{self, ErrorKind, Read, Seek, Write};

pub(super) const MAGIC: &[u8; 4] = b"NDK1";
pub(super) const OP_PUT: u8 = 1;
pub(super) const OP_DELETE: u8 = 2;
pub(super) const HEADER_LEN: u64 = 4 + 1 + 4 + 8 + 8;

const MAX_KEY_LEN: u32 = 64 * 1024;
const MAX_VALUE_LEN: u64 = 64 * 1024 * 1024;

#[derive(Debug)]
pub(super) struct ScannedRecord {
    pub(super) op: u8,
    pub(super) key: String,
    pub(super) value_offset: u64,
    pub(super) value_len: u64,
    pub(super) next_offset: u64,
}

pub(super) fn read_record(file: &mut File, offset: u64) -> io::Result<Option<ScannedRecord>> {
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

pub(super) fn write_record(file: &mut File, op: u8, key: &[u8], value: &[u8]) -> io::Result<()> {
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

pub(super) fn validate_key_value(key: &[u8], value: &[u8]) -> io::Result<()> {
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
