#![no_main]
use libfuzzer_sys::fuzz_target;

// Fuzz target that tests solution parsing with arbitrary byte sequences.
//
// Using `&[u8]` instead of `&str` allows the fuzzer to generate:
// - Invalid UTF-8 sequences
// - Embedded null bytes
// - Arbitrary binary data mixed with text
//
// This catches bugs in the lexer/parser that only surface with non-textual input.
fuzz_target!(|data: &[u8]| {
    // Use lossy conversion to handle any byte sequence,
    // but also try the raw approach for certain cases
    if let Ok(s) = std::str::from_utf8(data) {
        // Valid UTF-8 path — test normally
        let _ = solp::parse_str(s);
    } else {
        // Invalid UTF-8 — test with lossy conversion
        // This exercises the parser with replacement characters (U+FFFD)
        // which can uncover edge cases in string handling
        let lossy = String::from_utf8_lossy(data);
        let _ = solp::parse_str(&lossy);

        // Also test with only the valid prefix if any
        if let Some(valid_prefix) = find_valid_utf8_prefix(data) {
            if valid_prefix.len() >= 3 {
                let _ = solp::parse_str(std::str::from_utf8(valid_prefix).unwrap_or(""));
            }
        }
    }
});

// Find the longest valid UTF-8 prefix of a byte slice.
fn find_valid_utf8_prefix(data: &[u8]) -> Option<&[u8]> {
    let end = data.len();
    // Try progressively shorter prefixes until we find a valid UTF-8 boundary
    for len in (0..=end).rev() {
        if std::str::from_utf8(&data[..len]).is_ok() {
            return Some(&data[..len]);
        }
    }
    None
}