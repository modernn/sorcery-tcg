use serde_json::Value;

pub fn modifier_rows(unit: &Value, kind: &str) -> Vec<Value> {
    unit.get("temporaryModifiers")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter(|row| row.get("kind").and_then(Value::as_str) == Some(kind))
                .cloned()
                .collect()
        })
        .unwrap_or_default()
}

pub fn modifier_sources(unit: &Value, kind: &str) -> Value {
    Value::Array(
        modifier_rows(unit, kind)
            .into_iter()
            .filter_map(|row| row.get("sourceInstanceId").cloned())
            .collect(),
    )
}
