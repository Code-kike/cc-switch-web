//! Comment/structure-preserving JSON5 document editing.
//!
//! Wraps the `json-five` round-trip parser so that editing a single top-level
//! section of a JSON5 file leaves the rest of the file — comments, key order,
//! and whitespace — untouched. This is the same technique `openclaw_config`
//! uses; it is factored out here so `opencode_config` (and future JSON5 config
//! writers) can reuse it instead of re-serialising the whole file with strict,
//! sorted JSON (which destroys comments — see finding M33).
//!
//! `openclaw_config` predates this module and still carries its own copy of the
//! helpers; it can be migrated onto `Json5Document` in a later cleanup.

use crate::error::AppError;
use json_five::rt::parser::{
    from_str as rt_from_str, JSONKeyValuePair as RtJSONKeyValuePair,
    JSONObjectContext as RtJSONObjectContext, JSONText as RtJSONText, JSONValue as RtJSONValue,
    KeyValuePairContext as RtKeyValuePairContext,
};
use serde_json::Value;

/// A parsed JSON5 document whose root is an object, supporting comment-preserving
/// edits of its top-level sections.
pub struct Json5Document {
    text: RtJSONText,
}

impl Json5Document {
    /// Parse JSON5 source into a round-trip document.
    pub fn parse(source: &str) -> Result<Self, AppError> {
        let text = rt_from_str(source).map_err(|e| {
            AppError::Config(format!("Failed to parse JSON5 document: {}", e.message))
        })?;
        Ok(Self { text })
    }

    /// Insert or replace a top-level `key` with `value`, preserving the rest of
    /// the document (comments, other keys, ordering).
    pub fn set_root_section(&mut self, key: &str, value: &Value) -> Result<(), AppError> {
        let RtJSONValue::JSONObject {
            key_value_pairs,
            context,
        } = &mut self.text.value
        else {
            return Err(AppError::Config(
                "JSON5 document root must be an object".to_string(),
            ));
        };

        if key_value_pairs.is_empty()
            && context
                .as_ref()
                .map(|ctx| ctx.wsc.0.is_empty())
                .unwrap_or(true)
        {
            *context = Some(RtJSONObjectContext {
                wsc: ("\n  ".to_string(),),
            });
        }

        let leading_ws = context
            .as_ref()
            .map(|ctx| ctx.wsc.0.clone())
            .unwrap_or_default();
        let entry_separator_ws = derive_entry_separator(&leading_ws);
        let child_indent = extract_trailing_indent(&leading_ws);
        let new_value = value_to_rt_value(value, &child_indent)?;

        if let Some(existing) = key_value_pairs
            .iter_mut()
            .find(|pair| json5_key_name(&pair.key) == Some(key))
        {
            existing.value = new_value;
            return Ok(());
        }

        let new_pair = if let Some(last_pair) = key_value_pairs.last_mut() {
            let last_ctx = ensure_kvp_context(last_pair);
            let closing_ws = if let Some(after_comma) = last_ctx.wsc.3.clone() {
                last_ctx.wsc.3 = Some(entry_separator_ws.clone());
                after_comma
            } else {
                let closing_ws = std::mem::take(&mut last_ctx.wsc.2);
                last_ctx.wsc.3 = Some(entry_separator_ws.clone());
                closing_ws
            };

            make_root_pair(key, new_value, closing_ws)
        } else {
            make_root_pair(
                key,
                new_value,
                derive_closing_ws_from_separator(&leading_ws),
            )
        };

        key_value_pairs.push(new_pair);
        Ok(())
    }

    /// Remove a top-level `key`. Returns `true` if the key existed.
    ///
    /// A trailing comma may remain on the new last pair — that is valid JSON5,
    /// and callers should round-trip-verify the rendered output regardless.
    pub fn remove_root_section(&mut self, key: &str) -> bool {
        let RtJSONValue::JSONObject {
            key_value_pairs, ..
        } = &mut self.text.value
        else {
            return false;
        };

        if let Some(index) = key_value_pairs
            .iter()
            .position(|pair| json5_key_name(&pair.key) == Some(key))
        {
            key_value_pairs.remove(index);
            true
        } else {
            false
        }
    }

    /// Top-level keys, in document order.
    pub fn root_keys(&self) -> Vec<String> {
        match &self.text.value {
            RtJSONValue::JSONObject {
                key_value_pairs, ..
            } => key_value_pairs
                .iter()
                .filter_map(|pair| json5_key_name(&pair.key).map(str::to_string))
                .collect(),
            _ => Vec::new(),
        }
    }

    /// Render the (possibly edited) document back to JSON5 source text.
    pub fn to_source(&self) -> String {
        self.text.to_string()
    }
}

fn ensure_kvp_context(pair: &mut RtJSONKeyValuePair) -> &mut RtKeyValuePairContext {
    pair.context.get_or_insert_with(|| RtKeyValuePairContext {
        wsc: (String::new(), " ".to_string(), String::new(), None),
    })
}

fn extract_trailing_indent(separator_ws: &str) -> String {
    separator_ws
        .rsplit_once('\n')
        .map(|(_, tail)| tail.to_string())
        .unwrap_or_default()
}

fn derive_closing_ws_from_separator(separator_ws: &str) -> String {
    let Some((prefix, indent)) = separator_ws.rsplit_once('\n') else {
        return String::new();
    };

    let reduced_indent = if indent.ends_with('\t') {
        &indent[..indent.len().saturating_sub(1)]
    } else if indent.ends_with("  ") {
        &indent[..indent.len().saturating_sub(2)]
    } else if indent.ends_with(' ') {
        &indent[..indent.len().saturating_sub(1)]
    } else {
        indent
    };

    format!("{prefix}\n{reduced_indent}")
}

fn derive_entry_separator(leading_ws: &str) -> String {
    if leading_ws.is_empty() {
        return String::new();
    }

    if leading_ws.contains('\n') {
        return format!("\n{}", extract_trailing_indent(leading_ws));
    }

    String::new()
}

fn value_to_rt_value(value: &Value, parent_indent: &str) -> Result<RtJSONValue, AppError> {
    // `json-five` 0.3.1 can panic when pretty-printing nested empty maps/arrays.
    // Serialize with `serde_json` instead; the resulting JSON is valid JSON5 and
    // can still be parsed back into the round-trip AST we use for insertion.
    let source = serde_json::to_string_pretty(value)
        .map_err(|e| AppError::Config(format!("Failed to serialize JSON section: {e}")))?;

    let adjusted = reindent_json5_block(&source, parent_indent);
    let text = rt_from_str(&adjusted).map_err(|e| {
        AppError::Config(format!(
            "Failed to parse generated JSON5 section: {}",
            e.message
        ))
    })?;
    Ok(text.value)
}

fn reindent_json5_block(source: &str, parent_indent: &str) -> String {
    let normalized = normalize_json_five_output(source);
    if parent_indent.is_empty() || !normalized.contains('\n') {
        return normalized;
    }

    let mut lines = normalized.lines();
    let Some(first_line) = lines.next() else {
        return String::new();
    };

    let mut result = String::from(first_line);
    for line in lines {
        result.push('\n');
        result.push_str(parent_indent);
        result.push_str(line);
    }
    result
}

fn normalize_json_five_output(source: &str) -> String {
    source.replace("\\/", "/")
}

fn make_root_pair(key: &str, value: RtJSONValue, closing_ws: String) -> RtJSONKeyValuePair {
    RtJSONKeyValuePair {
        key: make_json5_key(key),
        value,
        context: Some(RtKeyValuePairContext {
            wsc: (String::new(), " ".to_string(), closing_ws, None),
        }),
    }
}

fn make_json5_key(key: &str) -> RtJSONValue {
    if is_identifier_key(key) {
        RtJSONValue::Identifier(key.to_string())
    } else {
        RtJSONValue::DoubleQuotedString(key.to_string())
    }
}

fn is_identifier_key(key: &str) -> bool {
    let mut chars = key.chars();
    let Some(first) = chars.next() else {
        return false;
    };

    matches!(first, 'a'..='z' | 'A'..='Z' | '_' | '$')
        && chars.all(|ch| matches!(ch, 'a'..='z' | 'A'..='Z' | '0'..='9' | '_' | '$'))
}

fn json5_key_name(key: &RtJSONValue) -> Option<&str> {
    match key {
        RtJSONValue::Identifier(name)
        | RtJSONValue::DoubleQuotedString(name)
        | RtJSONValue::SingleQuotedString(name) => Some(name),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn appends_new_root_section_preserving_comments() {
        let source = "{\n  // keep me\n  $schema: \"https://opencode.ai/config.json\",\n}\n";
        let mut doc = Json5Document::parse(source).unwrap();
        doc.set_root_section("provider", &json!({ "myprov": { "npm": "x" } }))
            .unwrap();

        let out = doc.to_source();
        assert!(out.contains("// keep me"), "comment must survive: {out}");
        assert!(out.contains("$schema"));
        assert!(out.contains("provider"));

        let parsed: Value = json5::from_str(&out).unwrap();
        assert_eq!(parsed["$schema"], json!("https://opencode.ai/config.json"));
        assert_eq!(parsed["provider"]["myprov"]["npm"], json!("x"));
    }

    #[test]
    fn replaces_existing_root_section_value() {
        let source = "{\n  // c\n  provider: { a: { npm: \"old\" } },\n}\n";
        let mut doc = Json5Document::parse(source).unwrap();
        doc.set_root_section("provider", &json!({ "a": { "npm": "new" } }))
            .unwrap();

        let out = doc.to_source();
        assert!(out.contains("// c"));
        let parsed: Value = json5::from_str(&out).unwrap();
        assert_eq!(parsed["provider"]["a"]["npm"], json!("new"));
    }

    #[test]
    fn removes_root_section_yielding_valid_json5() {
        let source = "{\n  // c\n  $schema: \"s\",\n  plugin: [\"p\"],\n}\n";
        let mut doc = Json5Document::parse(source).unwrap();
        assert!(doc.remove_root_section("plugin"));
        assert!(!doc.remove_root_section("missing"));

        let out = doc.to_source();
        let parsed: Value = json5::from_str(&out).unwrap();
        assert!(parsed.get("plugin").is_none());
        assert_eq!(parsed["$schema"], json!("s"));
        assert!(out.contains("// c"));
    }

    #[test]
    fn root_keys_in_order() {
        let doc = Json5Document::parse("{ a: 1, b: 2, c: 3 }").unwrap();
        assert_eq!(doc.root_keys(), vec!["a", "b", "c"]);
    }
}
