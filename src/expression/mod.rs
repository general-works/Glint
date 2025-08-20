use serde_json::Value;
use uuid::Uuid;

use crate::execution::NodeOutput;

pub fn is_expression(expression: &str) -> bool {
    expression.starts_with('=')
}

/// Extract value from expression
/// Expression format: =${node_id}.json_path
/// Example: =${8e1f2a34-5c67-4b89-a1d2-3f4e5a6b7c8d}.user.name
/// Or: =${8e1f2a34-5c67-4b89-a1d2-3f4e5a6b7c8d}.items[0].title
pub fn extract_value(expression: &str, input: &NodeOutput) -> Option<Value> {
    // Check if expression starts with =
    if !expression.starts_with('=') {
        return None;
    }

    // Remove leading = symbol
    let expr = &expression[1..];

    // Parse node ID and path
    let (node_id, json_path) = parse_expression(expr)?;

    // Get corresponding node value from input
    let node_value = input.get(&node_id)?;

    // Navigate JSON value according to path
    navigate_json_path(node_value, &json_path)
}

/// Parse expression, extract node ID and JSON path
/// Input: ${node_id}.json_path
/// Returns: (node_id, json_path)
fn parse_expression(expr: &str) -> Option<(Uuid, String)> {
    // Check if it starts with ${
    if !expr.starts_with("${") {
        return None;
    }

    // Find position of }
    let close_brace = expr.find('}')?;

    // Extract node ID string
    let node_id_str = &expr[2..close_brace];
    let node_id = Uuid::parse_str(node_id_str).ok()?;

    // Extract JSON path (if exists)
    let json_path =
        if close_brace + 1 < expr.len() && expr.chars().nth(close_brace + 1) == Some('.') {
            expr[close_brace + 2..].to_string()
        } else {
            String::new()
        };

    Some((node_id, json_path))
}

/// Validate if expression is legal
/// Check expression format, node ID validity and JSON path syntax
pub fn validate_expression(expression: &str) -> Result<(), String> {
    // Check if expression starts with =
    if !expression.starts_with('=') {
        return Err("Expression must start with '='".to_string());
    }

    // Remove leading = symbol
    let expr = &expression[1..];

    // Check if it starts with ${
    if !expr.starts_with("${") {
        return Err("Expression must contain '${node_id}' format".to_string());
    }

    // Find position of }
    let close_brace = expr
        .find('}')
        .ok_or_else(|| "Missing closing '}' symbol".to_string())?;

    // Check node ID part
    let node_id_str = &expr[2..close_brace];
    if node_id_str.is_empty() {
        return Err("Node ID cannot be empty".to_string());
    }

    // Validate if node ID is a valid UUID
    Uuid::parse_str(node_id_str).map_err(|_| format!("Invalid node ID format: {}", node_id_str))?;

    // Extract JSON path part
    let remaining = &expr[close_brace + 1..];

    // If there's a path part, it must start with .
    if !remaining.is_empty() {
        if !remaining.starts_with('.') {
            return Err("JSON path must start with '.'".to_string());
        }

        // Check if there's only a . without path content
        if remaining == "." {
            return Err("JSON path cannot be just a '.'".to_string());
        }

        let json_path = &remaining[1..];
        validate_json_path(json_path)?;
    }

    Ok(())
}

/// Validate JSON path syntax
fn validate_json_path(path: &str) -> Result<(), String> {
    if path.is_empty() {
        return Ok(());
    }

    // Check if it starts or ends with invalid characters
    if path.starts_with('.') || path.starts_with('[') {
        return Err("JSON path cannot start with '.' or '['".to_string());
    }

    if path.ends_with('.') {
        return Err("JSON path cannot end with '.'".to_string());
    }

    let mut current_path = path;
    let mut expecting_key = true; // Mark whether currently expecting a key name

    while !current_path.is_empty() {
        if current_path.starts_with('[') {
            if expecting_key {
                return Err("Array index must be preceded by a property name".to_string());
            }

            // Find closing bracket
            let close_bracket = current_path
                .find(']')
                .ok_or_else(|| "Missing closing ']' symbol".to_string())?;

            // Check index content
            let index_str = &current_path[1..close_bracket];
            if index_str.is_empty() {
                return Err("Array index cannot be empty".to_string());
            }

            // Validate if index is a valid number
            index_str
                .parse::<usize>()
                .map_err(|_| format!("Invalid array index: {}", index_str))?;

            // Update path
            current_path = &current_path[close_bracket + 1..];
            expecting_key = false; // After array index can directly follow . or [

            // If there's more content after, check if it starts with . or [
            if !current_path.is_empty() {
                if current_path.starts_with('.') {
                    current_path = &current_path[1..];
                    expecting_key = true; // After . must be a key name
                } else if !current_path.starts_with('[') {
                    return Err("After array index can only follow '.' or another '['".to_string());
                }
            }
        } else {
            // Handle property names
            if !expecting_key {
                return Err("Expected '[' here instead of property name".to_string());
            }

            let next_separator = current_path.find(|c| c == '.' || c == '[');
            let (key, remaining_path) = if let Some(pos) = next_separator {
                (&current_path[..pos], &current_path[pos..])
            } else {
                (current_path, "")
            };

            // Validate key name
            if key.is_empty() {
                return Err("Property name cannot be empty".to_string());
            }

            // Check if key name contains invalid characters
            if !is_valid_json_key(key) {
                return Err(format!("Invalid property name: {}", key));
            }

            // Update path
            current_path = if remaining_path.starts_with('.') {
                expecting_key = true;
                &remaining_path[1..]
            } else {
                expecting_key = false;
                remaining_path
            };
        }
    }

    Ok(())
}

/// Check if it's a valid JSON key name
/// Simple validation rule: cannot contain special characters
fn is_valid_json_key(key: &str) -> bool {
    !key.is_empty() && !key.contains(|c: char| c.is_control() || "[]{}().".contains(c))
}

/// Navigate JSON value according to path
/// Supports object property access (.key) and array index access ([index])
fn navigate_json_path(value: &Value, path: &str) -> Option<Value> {
    if path.is_empty() {
        return Some(value.clone());
    }

    let mut current_value = value;
    let mut current_path = path;

    while !current_path.is_empty() {
        // Check if it's array index access
        if current_path.starts_with('[') {
            let close_bracket = current_path.find(']')?;
            let index_str = &current_path[1..close_bracket];
            let index = index_str.parse::<usize>().ok()?;

                    // Ensure current value is an array
        let array = current_value.as_array()?;
        if index >= array.len() {
            return None;
        }

        current_value = &array[index];

        // Update path, skip processed parts
        current_path = &current_path[close_bracket + 1..];

        // If there's more path after, it should start with .
        if !current_path.is_empty() && current_path.starts_with('.') {
            current_path = &current_path[1..];
        }
    } else {
        // Object property access
        let next_separator = current_path.find(|c| c == '.' || c == '[');
        let (key, remaining_path) = if let Some(pos) = next_separator {
            (&current_path[..pos], &current_path[pos..])
        } else {
            (current_path, "")
        };

        // Ensure current value is an object
        let obj = current_value.as_object()?;
        current_value = obj.get(key)?;

        // Update path
        current_path = if remaining_path.starts_with('.') {
            &remaining_path[1..]
        } else {
            remaining_path
        };
    }
    }

    Some(current_value.clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::collections::HashMap;

    #[test]
    fn test_parse_expression() {
        let expr = "${8e1f2a34-5c67-4b89-a1d2-3f4e5a6b7c8d}.user.name";
        let (node_id, path) = parse_expression(expr).unwrap();

        assert_eq!(node_id.to_string(), "8e1f2a34-5c67-4b89-a1d2-3f4e5a6b7c8d");
        assert_eq!(path, "user.name");
    }

    #[test]
    fn test_navigate_json_path_object() {
        let json = json!({
            "user": {
                "name": "John Doe",
                "email": "john@example.com"
            }
        });

        let result = navigate_json_path(&json, "user.name").unwrap();
        assert_eq!(result, json!("John Doe"));
    }

    #[test]
    fn test_navigate_json_path_array() {
        let json = json!({
            "items": [
                {"title": "First Item"},
                {"title": "Second Item"}
            ]
        });

        let result = navigate_json_path(&json, "items[0].title").unwrap();
        assert_eq!(result, json!("First Item"));
    }

    #[test]
    fn test_extract_value_complete() {
        let node_id = Uuid::parse_str("8e1f2a34-5c67-4b89-a1d2-3f4e5a6b7c8d").unwrap();
        let mut input = HashMap::new();
        input.insert(
            node_id,
            json!({
                "user": "John Doe",
                "email": "john.doe@example.com",
                "contacts": [
                    {"type": "phone", "value": "123-456-7890"},
                    {"type": "email", "value": "john@work.com"}
                ]
            }),
        );

        // Test simple property access
        let expr = "=${8e1f2a34-5c67-4b89-a1d2-3f4e5a6b7c8d}.user";
        let result = extract_value(expr, &input).unwrap();
        assert_eq!(result, json!("John Doe"));

        // Test array access
        let expr = "=${8e1f2a34-5c67-4b89-a1d2-3f4e5a6b7c8d}.contacts[0].value";
        let result = extract_value(expr, &input).unwrap();
        assert_eq!(result, json!("123-456-7890"));

        // Test getting entire object
        let expr = "=${8e1f2a34-5c67-4b89-a1d2-3f4e5a6b7c8d}";
        let result = extract_value(expr, &input).unwrap();
        assert_eq!(result, input.get(&node_id).unwrap().clone());
    }

    #[test]
    fn test_validate_expression_valid() {
        // Test valid expressions
        assert!(validate_expression("=${8e1f2a34-5c67-4b89-a1d2-3f4e5a6b7c8d}").is_ok());
        assert!(validate_expression("=${8e1f2a34-5c67-4b89-a1d2-3f4e5a6b7c8d}.user").is_ok());
        assert!(validate_expression("=${8e1f2a34-5c67-4b89-a1d2-3f4e5a6b7c8d}.user.name").is_ok());
        assert!(
            validate_expression("=${8e1f2a34-5c67-4b89-a1d2-3f4e5a6b7c8d}.contacts[0]").is_ok()
        );
        assert!(
            validate_expression("=${8e1f2a34-5c67-4b89-a1d2-3f4e5a6b7c8d}.contacts[0].value")
                .is_ok()
        );
        assert!(
            validate_expression("=${8e1f2a34-5c67-4b89-a1d2-3f4e5a6b7c8d}.data[0][1].name").is_ok()
        );
    }

    #[test]
    fn test_validate_expression_invalid_format() {
        // Test invalid formats
        assert!(validate_expression("${8e1f2a34-5c67-4b89-a1d2-3f4e5a6b7c8d}").is_err());
        assert!(validate_expression("=8e1f2a34-5c67-4b89-a1d2-3f4e5a6b7c8d").is_err());
        assert!(validate_expression("={8e1f2a34-5c67-4b89-a1d2-3f4e5a6b7c8d}").is_err());
        assert!(validate_expression("=${8e1f2a34-5c67-4b89-a1d2-3f4e5a6b7c8d").is_err());
    }

    #[test]
    fn test_validate_expression_invalid_node_id() {
        // Test invalid node IDs
        assert!(validate_expression("=${}").is_err());
        assert!(validate_expression("=${invalid-uuid}").is_err());
        assert!(validate_expression("=${123-456-789}").is_err());
    }

    #[test]
    fn test_validate_expression_invalid_path() {
        let base = "=${8e1f2a34-5c67-4b89-a1d2-3f4e5a6b7c8d}";

        // Test invalid paths
        assert!(validate_expression(&format!("{}..", base)).is_err());
        assert!(validate_expression(&format!("{}.", base)).is_err());
        assert!(validate_expression(&format!("{}.user.", base)).is_err());
        assert!(validate_expression(&format!("{}.user[]", base)).is_err());
        assert!(validate_expression(&format!("{}.user[abc]", base)).is_err());
        assert!(validate_expression(&format!("{}.user[-1]", base)).is_err());
        assert!(validate_expression(&format!("{}.user[0", base)).is_err());
        assert!(validate_expression(&format!("{}.user.name[", base)).is_err());
    }

    #[test]
    fn test_validate_json_path() {
        // Test valid paths
        assert!(validate_json_path("").is_ok());
        assert!(validate_json_path("user").is_ok());
        assert!(validate_json_path("user.name").is_ok());
        assert!(validate_json_path("contacts[0]").is_ok());
        assert!(validate_json_path("contacts[0].value").is_ok());
        assert!(validate_json_path("data[0][1].name").is_ok());

        // Test invalid paths
        assert!(validate_json_path(".").is_err());
        assert!(validate_json_path("user.").is_err());
        assert!(validate_json_path("[0]").is_err());
        assert!(validate_json_path("user[]").is_err());
        assert!(validate_json_path("user[abc]").is_err());
    }

    #[test]
    fn test_is_valid_json_key() {
        // Test valid key names
        assert!(is_valid_json_key("user"));
        assert!(is_valid_json_key("userName"));
        assert!(is_valid_json_key("user_name"));
        assert!(is_valid_json_key("user123"));
        assert!(is_valid_json_key("name-with-dash"));

        // Test invalid key names
        assert!(!is_valid_json_key(""));
        assert!(!is_valid_json_key("user.name"));
        assert!(!is_valid_json_key("user[0]"));
        assert!(!is_valid_json_key("user{name}"));
        assert!(!is_valid_json_key("user()"));
    }
}
