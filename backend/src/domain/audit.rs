use crate::domain::types::{Message, Role};
use serde_json::{Map, Value};

const REDACTED: &str = "***REDACTED***";

pub fn redact_json(value: &Value) -> Value {
    match value {
        Value::Object(object) => {
            let mut mapped = Map::with_capacity(object.len());
            for (key, value) in object {
                if is_sensitive_key(key) {
                    mapped.insert(key.clone(), Value::String(REDACTED.to_string()));
                } else {
                    mapped.insert(key.clone(), redact_json(value));
                }
            }
            Value::Object(mapped)
        }
        Value::Array(array) => Value::Array(array.iter().map(redact_json).collect()),
        _ => value.clone(),
    }
}

pub fn redact_raw_json(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    match serde_json::from_str::<Value>(trimmed) {
        Ok(value) => serde_json::to_string_pretty(&redact_json(&value))
            .map(|text| format!("{text}\n"))
            .unwrap_or_else(|_| format!("{REDACTED}\n")),
        Err(_) => "<non-json payload redacted>\n".to_string(),
    }
}

pub fn total_message_chars(messages: &[Message]) -> usize {
    messages
        .iter()
        .map(|message| message.content.chars().count())
        .sum()
}

pub fn role_counts(messages: &[Message]) -> [usize; 4] {
    let mut counts = [0usize; 4];
    for message in messages {
        match message.role {
            Role::System => counts[0] += 1,
            Role::User => counts[1] += 1,
            Role::Assistant => counts[2] += 1,
            Role::Tool => counts[3] += 1,
        }
    }
    counts
}

fn is_sensitive_key(key: &str) -> bool {
    let lowered = key.to_ascii_lowercase();
    [
        "api_key",
        "apikey",
        "secret",
        "token",
        "password",
        "authorization",
    ]
    .iter()
    .any(|needle| lowered.contains(needle))
}
