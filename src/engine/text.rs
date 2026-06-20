pub(super) fn approximate_token_count(text: &str) -> usize {
    let word_count = text.split_whitespace().count();
    if word_count > 0 {
        word_count
    } else {
        text.chars().count().div_ceil(2)
    }
}

pub(super) fn compact(text: &str, max_chars: usize) -> String {
    let mut out = text.chars().take(max_chars).collect::<String>();
    if text.chars().count() > max_chars {
        out.push_str("...");
    }
    out
}

pub(super) fn hash_char(ch: char) -> usize {
    let mut buffer = [0_u8; 4];
    let mut hash = 0xcbf29ce484222325_u64;

    for byte in ch.encode_utf8(&mut buffer).as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }

    hash as usize
}

pub(super) fn char_weight(ch: char) -> f32 {
    if ch.is_ascii_alphabetic() {
        1.0
    } else if ch.is_ascii_digit() {
        1.15
    } else if ch.is_ascii_punctuation() {
        0.35
    } else {
        1.25
    }
}
