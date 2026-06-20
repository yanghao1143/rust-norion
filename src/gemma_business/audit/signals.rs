mod denial;
mod handling;
mod protocol;

use denial::business_answer_contains_evasive_denial_impl;
use handling::business_answer_contains_handling_signal_impl;
use protocol::business_answer_contains_protocol_leak_impl;

pub fn business_answer_contains_protocol_leak(answer: &str, lower: &str) -> bool {
    business_answer_contains_protocol_leak_impl(answer, lower)
}

pub fn business_answer_contains_evasive_denial(answer: &str, lower: &str) -> bool {
    business_answer_contains_evasive_denial_impl(answer, lower)
}

pub fn business_answer_contains_handling_signal(answer: &str, lower: &str) -> bool {
    business_answer_contains_handling_signal_impl(answer, lower)
}

pub fn business_answer_contains_signal(answer: &str, lower: &str, signal: &str) -> bool {
    if signal.is_ascii() {
        lower.contains(&signal.to_ascii_lowercase())
    } else {
        answer.contains(signal)
    }
}
