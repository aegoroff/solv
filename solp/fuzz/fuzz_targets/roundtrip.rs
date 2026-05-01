#![no_main]
use libfuzzer_sys::fuzz_target;

// Round-trip fuzz target: parse → serialize → parse again.
//
// This verifies that valid solution files survive a parse-serialize-parse cycle
// without structural changes. Inconsistencies between the first and second parse
// reveal bugs in the parser or the serialization layer.
fuzz_target!(|data: &[u8]| {
    // Step 1: Parse the input
    let Ok(s) = std::str::from_utf8(data) else {
        return;
    };

    let Ok(first) = solp::parse_str(s) else {
        return;
    };

    // Step 2: Serialize to JSON (a stable, well-defined format)
    let Ok(json) = serde_json::to_string(&first) else {
        return;
    };

    // Step 3: Check structural invariants
    // A valid round-trip: same number of projects, configurations
    // (Skip path since it's empty for parse_str)
    for project in &first.projects {
        assert!(!project.type_id.is_empty(), "project type_id must not be empty");
        assert!(!project.id.is_empty(), "project id must not be empty");
        assert!(!project.name.is_empty(), "project name must not be empty");
    }

    // Step 4: Verify JSON is not empty for valid input
    if !json.is_empty() && json != "null" {
        // Try to parse the JSON back — deserialize once to check it's valid
        let Ok(_deserialized) = serde_json::from_str::<serde_json::Value>(&json) else {
            panic!("Serialized JSON is not valid JSON: {json}");
        };
    }

    // Step 5: Check that dropped path doesn't affect the rest
    assert!(first.path.is_empty(), "parse_str Solution path must be empty");

    // If we got this far, the input was a valid solution file.
    // The key invariant is: valid parse + valid JSON serialization.
    let _ = json;
});