use std::collections::HashSet;

/// Evaluate deny-first permission based on member tags and project permission settings.
///
/// Algorithm:
/// 1. Collect deny rules for all matching tags -> union into deny set
/// 2. Collect allow rules for all matching tags -> union into allow set
/// 3. Deny set takes priority over allow set
/// 4. If item is not in final allowed set -> denied
///
/// Returns `true` if the requested item is allowed.
pub fn evaluate_deny_first(
    member_tag_names: &[String],
    permission_settings: &serde_json::Value,
    dimension: &str,
    requested_item: &str,
) -> bool {
    let Some(dimension_obj) = permission_settings.get(dimension) else {
        return false; // No rules for this dimension -> default deny
    };

    let mut deny_set = HashSet::new();
    let mut allow_set = HashSet::new();

    for tag_name in member_tag_names {
        if let Some(tag_rules) = dimension_obj.get(tag_name) {
            collect_rules(tag_rules, &mut deny_set, &mut allow_set);
        }
    }

    // Also check wildcard "*" tag
    if let Some(wildcard_rules) = dimension_obj.get("*") {
        collect_rules(wildcard_rules, &mut deny_set, &mut allow_set);
    }

    // Deny takes priority
    if deny_set.contains(requested_item) || deny_set.contains("*") {
        return false;
    }

    // Check allow
    allow_set.contains(requested_item) || allow_set.contains("*")
}

fn collect_rules(
    tag_rules: &serde_json::Value,
    deny_set: &mut HashSet<String>,
    allow_set: &mut HashSet<String>,
) {
    if let Some(deny_arr) = tag_rules.get("deny").and_then(|v| v.as_array()) {
        for item in deny_arr {
            if let Some(s) = item.as_str() {
                deny_set.insert(s.to_string());
            }
        }
    }
    if let Some(allow_arr) = tag_rules.get("allow").and_then(|v| v.as_array()) {
        for item in allow_arr {
            if let Some(s) = item.as_str() {
                allow_set.insert(s.to_string());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn deny_overrides_allow() {
        let settings = json!({
            "task_visibility": {
                "dev": {
                    "allow": ["taskA", "taskB"],
                    "deny": ["taskA"]
                }
            }
        });
        let tags = vec!["dev".to_string()];
        assert!(!evaluate_deny_first(
            &tags,
            &settings,
            "task_visibility",
            "taskA"
        ));
        assert!(evaluate_deny_first(
            &tags,
            &settings,
            "task_visibility",
            "taskB"
        ));
    }

    #[test]
    fn allow_no_deny() {
        let settings = json!({
            "task_visibility": {
                "dev": {
                    "allow": ["taskA"]
                }
            }
        });
        let tags = vec!["dev".to_string()];
        assert!(evaluate_deny_first(
            &tags,
            &settings,
            "task_visibility",
            "taskA"
        ));
        assert!(!evaluate_deny_first(
            &tags,
            &settings,
            "task_visibility",
            "taskB"
        ));
    }

    #[test]
    fn wildcard_tag() {
        let settings = json!({
            "task_visibility": {
                "*": {
                    "allow": ["taskA"]
                }
            }
        });
        // Even with no matching tag names, wildcard applies
        let tags = vec!["random_tag".to_string()];
        assert!(evaluate_deny_first(
            &tags,
            &settings,
            "task_visibility",
            "taskA"
        ));
    }

    #[test]
    fn no_rules_default_deny() {
        let settings = json!({});
        let tags = vec!["dev".to_string()];
        assert!(!evaluate_deny_first(
            &tags,
            &settings,
            "task_visibility",
            "taskA"
        ));
    }

    #[test]
    fn multi_tag_deny_first() {
        let settings = json!({
            "task_visibility": {
                "dev": {
                    "allow": ["taskA"]
                },
                "intern": {
                    "deny": ["taskA"]
                }
            }
        });
        let tags = vec!["dev".to_string(), "intern".to_string()];
        // "intern" denies taskA, so it should be denied even though "dev" allows it
        assert!(!evaluate_deny_first(
            &tags,
            &settings,
            "task_visibility",
            "taskA"
        ));
    }

    #[test]
    fn wildcard_allow() {
        let settings = json!({
            "tool_permissions": {
                "admin": {
                    "allow": ["*"]
                }
            }
        });
        let tags = vec!["admin".to_string()];
        assert!(evaluate_deny_first(
            &tags,
            &settings,
            "tool_permissions",
            "any_tool"
        ));
    }

    #[test]
    fn wildcard_deny() {
        let settings = json!({
            "tool_permissions": {
                "restricted": {
                    "deny": ["*"]
                }
            }
        });
        let tags = vec!["restricted".to_string()];
        assert!(!evaluate_deny_first(
            &tags,
            &settings,
            "tool_permissions",
            "any_tool"
        ));
    }
}
