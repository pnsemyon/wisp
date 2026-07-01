//! The `Profile` data model: a named connection made up of one or more
//! sing-box outbounds.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// A user-visible connection profile. Each outbound is stored verbatim as
/// sing-box JSON (the formats we import already produce sing-box-shaped
/// outbounds), so `wisp-core` never needs to re-serialize protocol details.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub id: String,
    pub name: String,
    pub outbounds: Vec<Value>,
    pub active_tag: Option<String>,
}

impl Profile {
    /// Build a new profile from a name and a set of outbounds. `id` is
    /// derived deterministically from a slug of `name` (no randomness),
    /// with `existing_ids` used to disambiguate by appending a numeric
    /// suffix if the slug is already taken.
    pub fn new(name: impl Into<String>, outbounds: Vec<Value>, existing_ids: &[String]) -> Self {
        let name = name.into();
        let base = slugify(&name);
        let id = unique_id(&base, existing_ids);
        let active_tag = outbounds.first().and_then(tag_of);
        Profile {
            id,
            name,
            outbounds,
            active_tag,
        }
    }

    /// Every outbound's `"tag"` field, in order, skipping outbounds that
    /// have no tag.
    pub fn tags(&self) -> Vec<String> {
        self.outbounds.iter().filter_map(tag_of).collect()
    }
}

fn tag_of(outbound: &Value) -> Option<String> {
    outbound
        .get("tag")
        .and_then(Value::as_str)
        .map(|s| s.to_string())
}

/// Lowercase, ASCII-only slug: non-alphanumeric runs collapse to a single
/// `-`, leading/trailing `-` trimmed. Falls back to `"profile"` if the
/// input has no alphanumeric characters.
pub fn slugify(name: &str) -> String {
    let mut slug = String::with_capacity(name.len());
    let mut last_was_dash = false;
    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch.to_ascii_lowercase());
            last_was_dash = false;
        } else if !last_was_dash && !slug.is_empty() {
            slug.push('-');
            last_was_dash = true;
        }
    }
    while slug.ends_with('-') {
        slug.pop();
    }
    if slug.is_empty() {
        "profile".to_string()
    } else {
        slug
    }
}

fn unique_id(base: &str, existing_ids: &[String]) -> String {
    if !existing_ids.iter().any(|id| id == base) {
        return base.to_string();
    }
    let mut idx = 1;
    loop {
        let candidate = format!("{base}-{idx}");
        if !existing_ids.iter().any(|id| id == &candidate) {
            return candidate;
        }
        idx += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn slugify_basic() {
        assert_eq!(slugify("Bulgaria, Sophia-7w1t0rtt5a"), "bulgaria-sophia-7w1t0rtt5a");
        assert_eq!(slugify("   "), "profile");
        assert_eq!(slugify("A/B/C"), "a-b-c");
    }

    #[test]
    fn unique_id_disambiguates() {
        let existing = vec!["foo".to_string()];
        assert_eq!(unique_id("foo", &existing), "foo-1");
        assert_eq!(unique_id("bar", &existing), "bar");
    }

    #[test]
    fn tags_reads_outbound_tags() {
        let profile = Profile::new(
            "test",
            vec![json!({"type": "vless", "tag": "a"}), json!({"type": "direct"})],
            &[],
        );
        assert_eq!(profile.tags(), vec!["a".to_string()]);
    }
}
