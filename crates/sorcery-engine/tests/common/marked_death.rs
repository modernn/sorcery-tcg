use serde_json::Value;

pub fn assert_live_marked_before_cemetery(state: &Value, instance_ids: &[String]) {
    assert!(!instance_ids.is_empty(), "expected marked cohort");
    let units = state["realm"]["units"].as_array().expect("realm units");
    for instance_id in instance_ids {
        let unit = units
            .iter()
            .find(|unit| unit["instanceId"] == instance_id.as_str())
            .expect("marked body remains in realm during Deathrites");
        assert_eq!(unit["deathMarked"], true, "marked body {instance_id}");
        let owner = unit["owner"].as_str().expect("immutable card owner");
        assert!(
            state["players"][owner]["cemetery"]
                .as_array()
                .expect("owner cemetery")
                .iter()
                .all(|card| card["instanceId"] != instance_id.as_str()),
            "marked body {instance_id} has not entered owner's cemetery"
        );
    }
}
