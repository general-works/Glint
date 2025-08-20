use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;

use crate::execution::NodeOutput;
use crate::node::{INode, NodeId, Error};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransformNode {
    pub id: NodeId,
    pub name: String,
    pub description: Option<String>,
    pub parameters: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransformParams {
    pub transformations: Vec<Transformation>,
    pub mode: String, // "each" or "batch"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transformation {
    pub operation: String, // "map", "filter", "sort", "group", "rename", "add_field", "remove_field"
    pub field: Option<String>,
    pub new_field: Option<String>,
    pub value: Option<Value>,
    pub condition: Option<TransformCondition>,
    pub sort_key: Option<String>,
    pub sort_order: Option<String>, // "asc" or "desc"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransformCondition {
    pub field: String,
    pub operator: String,
    pub value: Value,
}

impl Default for TransformParams {
    fn default() -> Self {
        Self {
            transformations: vec![],
            mode: "each".to_string(),
        }
    }
}

impl INode for TransformNode {
    fn new(
        id: NodeId,
        name: String,
        _node_type: String,
        description: Option<String>,
        parameters: Option<Value>,
    ) -> Self {
        Self {
            id,
            name,
            description,
            parameters,
        }
    }

    fn id(&self) -> NodeId {
        self.id
    }

    fn name(&self) -> String {
        self.name.clone()
    }

    fn description(&self) -> Option<String> {
        self.description.clone()
    }

    fn parameter(&self) -> Option<Value> {
        self.parameters.clone()
    }

    fn execute(&self, input: &NodeOutput) -> Result<Value, Error> {
        let params = if let Some(params_value) = &self.parameters {
            serde_json::from_value::<TransformParams>(params_value.clone())
                .unwrap_or_default()
        } else {
            TransformParams::default()
        };

        // Combine all input data from previous nodes
        let input_data: Vec<Value> = input.values().cloned().collect();
        
        // If no input data, return empty array
        if input_data.is_empty() {
            return Ok(json!([]));
        }

        // Flatten all input arrays into a single collection
        let mut all_items = Vec::new();
        for item in input_data {
            if let Value::Array(items) = item {
                all_items.extend(items);
            } else {
                all_items.push(item);
            }
        }

        match params.mode.as_str() {
            "each" => self.transform_each(&params.transformations, all_items),
            "batch" => self.transform_batch(&params.transformations, all_items),
            _ => Err(Error::Validation(format!("Unsupported mode: {}", params.mode)))
        }
    }

    fn validate(&self) -> bool {
        if let Some(params_value) = &self.parameters {
            if let Ok(params) = serde_json::from_value::<TransformParams>(params_value.clone()) {
                return !params.transformations.is_empty();
            }
        }
        false
    }

    fn dependencies(&self) -> Vec<NodeId> {
        vec![]
    }
}

impl TransformNode {
    fn transform_each(&self, transformations: &[Transformation], items: Vec<Value>) -> Result<Value, Error> {
        let mut result = Vec::new();
        
        for item in items {
            let mut transformed_item = item;
            
            // Apply each transformation to the item
            for transformation in transformations {
                transformed_item = self.apply_transformation(transformation, transformed_item)?;
            }
            
            result.push(transformed_item);
        }
        
        Ok(Value::Array(result))
    }

    fn transform_batch(&self, transformations: &[Transformation], items: Vec<Value>) -> Result<Value, Error> {
        let mut result = items;
        
        // Apply each transformation to the entire batch
        for transformation in transformations {
            result = self.apply_batch_transformation(transformation, result)?;
        }
        
        Ok(Value::Array(result))
    }

    fn apply_transformation(&self, transformation: &Transformation, item: Value) -> Result<Value, Error> {
        match transformation.operation.as_str() {
            "add_field" => self.add_field(transformation, item),
            "remove_field" => self.remove_field(transformation, item),
            "rename_field" => self.rename_field(transformation, item),
            "map_value" => self.map_value(transformation, item),
            "filter" => {
                if self.passes_condition(transformation, &item)? {
                    Ok(item)
                } else {
                    Ok(Value::Null) // Will be filtered out later
                }
            }
            _ => Err(Error::Validation(format!("Unsupported transformation: {}", transformation.operation)))
        }
    }

    fn apply_batch_transformation(&self, transformation: &Transformation, items: Vec<Value>) -> Result<Vec<Value>, Error> {
        match transformation.operation.as_str() {
            "sort" => self.sort_items(transformation, items),
            "group" => self.group_items(transformation, items),
            "filter" => self.filter_items(transformation, items),
            _ => {
                // Apply single-item transformations to each item
                let mut result = Vec::new();
                for item in items {
                    let transformed = self.apply_transformation(transformation, item)?;
                    if !transformed.is_null() {
                        result.push(transformed);
                    }
                }
                Ok(result)
            }
        }
    }

    fn add_field(&self, transformation: &Transformation, mut item: Value) -> Result<Value, Error> {
        let new_field = transformation.new_field.as_ref()
            .ok_or_else(|| Error::Validation("new_field is required for add_field operation".to_string()))?;
        let value = transformation.value.as_ref()
            .ok_or_else(|| Error::Validation("value is required for add_field operation".to_string()))?;
        
        if let Value::Object(ref mut obj) = item {
            obj.insert(new_field.clone(), value.clone());
        }
        
        Ok(item)
    }

    fn remove_field(&self, transformation: &Transformation, mut item: Value) -> Result<Value, Error> {
        let field = transformation.field.as_ref()
            .ok_or_else(|| Error::Validation("field is required for remove_field operation".to_string()))?;
        
        if let Value::Object(ref mut obj) = item {
            obj.remove(field);
        }
        
        Ok(item)
    }

    fn rename_field(&self, transformation: &Transformation, mut item: Value) -> Result<Value, Error> {
        let old_field = transformation.field.as_ref()
            .ok_or_else(|| Error::Validation("field is required for rename_field operation".to_string()))?;
        let new_field = transformation.new_field.as_ref()
            .ok_or_else(|| Error::Validation("new_field is required for rename_field operation".to_string()))?;
        
        if let Value::Object(ref mut obj) = item {
            if let Some(value) = obj.remove(old_field) {
                obj.insert(new_field.clone(), value);
            }
        }
        
        Ok(item)
    }

    fn map_value(&self, transformation: &Transformation, mut item: Value) -> Result<Value, Error> {
        let field = transformation.field.as_ref()
            .ok_or_else(|| Error::Validation("field is required for map_value operation".to_string()))?;
        let new_value = transformation.value.as_ref()
            .ok_or_else(|| Error::Validation("value is required for map_value operation".to_string()))?;
        
        if let Value::Object(ref mut obj) = item {
            obj.insert(field.clone(), new_value.clone());
        }
        
        Ok(item)
    }

    fn sort_items(&self, transformation: &Transformation, mut items: Vec<Value>) -> Result<Vec<Value>, Error> {
        let sort_key = transformation.sort_key.as_ref()
            .ok_or_else(|| Error::Validation("sort_key is required for sort operation".to_string()))?;
        let sort_order = transformation.sort_order.as_ref().map(|s| s.as_str()).unwrap_or("asc");
        
        items.sort_by(|a, b| {
            let a_val = self.get_field_value(sort_key, a).unwrap_or(Value::Null);
            let b_val = self.get_field_value(sort_key, b).unwrap_or(Value::Null);
            
            let cmp = match (&a_val, &b_val) {
                (Value::Number(a_num), Value::Number(b_num)) => {
                    a_num.as_f64().partial_cmp(&b_num.as_f64()).unwrap_or(std::cmp::Ordering::Equal)
                }
                (Value::String(a_str), Value::String(b_str)) => a_str.cmp(b_str),
                _ => std::cmp::Ordering::Equal,
            };
            
            if sort_order == "desc" {
                cmp.reverse()
            } else {
                cmp
            }
        });
        
        Ok(items)
    }

    fn group_items(&self, transformation: &Transformation, items: Vec<Value>) -> Result<Vec<Value>, Error> {
        let group_key = transformation.field.as_ref()
            .ok_or_else(|| Error::Validation("field is required for group operation".to_string()))?;
        
        let mut groups: HashMap<String, Vec<Value>> = HashMap::new();
        
        for item in items {
            let key_value = self.get_field_value(group_key, &item).unwrap_or(Value::Null);
            let key = match key_value {
                Value::String(s) => s,
                Value::Number(n) => n.to_string(),
                Value::Bool(b) => b.to_string(),
                _ => "null".to_string(),
            };
            
            groups.entry(key).or_insert_with(Vec::new).push(item);
        }
        
        // Convert groups to result format
        let mut result = Vec::new();
        for (key, group_items) in groups {
            result.push(json!({
                "key": key,
                "items": group_items,
                "count": group_items.len()
            }));
        }
        
        Ok(result)
    }

    fn filter_items(&self, transformation: &Transformation, items: Vec<Value>) -> Result<Vec<Value>, Error> {
        let mut result = Vec::new();
        
        for item in items {
            if self.passes_condition(transformation, &item)? {
                result.push(item);
            }
        }
        
        Ok(result)
    }

    fn passes_condition(&self, transformation: &Transformation, item: &Value) -> Result<bool, Error> {
        let condition = transformation.condition.as_ref()
            .ok_or_else(|| Error::Validation("condition is required for filter operation".to_string()))?;
        
        let field_value = self.get_field_value(&condition.field, item)?;
        
        match condition.operator.as_str() {
            "equals" => Ok(field_value == condition.value),
            "notEquals" => Ok(field_value != condition.value),
            "contains" => {
                if let (Value::String(haystack), Value::String(needle)) = (&field_value, &condition.value) {
                    Ok(haystack.contains(needle))
                } else {
                    Ok(false)
                }
            }
            "greaterThan" => {
                match (&field_value, &condition.value) {
                    (Value::Number(a), Value::Number(b)) => {
                        if let (Some(a_f64), Some(b_f64)) = (a.as_f64(), b.as_f64()) {
                            Ok(a_f64 > b_f64)
                        } else {
                            Ok(false)
                        }
                    }
                    _ => Ok(false)
                }
            }
            "lessThan" => {
                match (&field_value, &condition.value) {
                    (Value::Number(a), Value::Number(b)) => {
                        if let (Some(a_f64), Some(b_f64)) = (a.as_f64(), b.as_f64()) {
                            Ok(a_f64 < b_f64)
                        } else {
                            Ok(false)
                        }
                    }
                    _ => Ok(false)
                }
            }
            _ => Err(Error::Validation(format!("Unsupported operator: {}", condition.operator)))
        }
    }

    fn get_field_value(&self, field_path: &str, data: &Value) -> Result<Value, Error> {
        let parts: Vec<&str> = field_path.split('.').collect();
        let mut current = data;

        for part in parts {
            if let Some(next) = current.get(part) {
                current = next;
            } else {
                return Ok(Value::Null);
            }
        }

        Ok(current.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn test_transform_node_creation() {
        let node = TransformNode::new(
            Uuid::new_v4(),
            "Test Transform".to_string(),
            "transform".to_string(),
            Some("Test description".to_string()),
            Some(json!({
                "transformations": [{
                    "operation": "add_field",
                    "new_field": "processed",
                    "value": true
                }],
                "mode": "each"
            })),
        );

        assert_eq!(node.name(), "Test Transform");
        assert!(node.validate());
    }

    #[test]
    fn test_transform_validation() {
        let node = TransformNode::new(
            Uuid::new_v4(),
            "Test Transform".to_string(),
            "transform".to_string(),
            None,
            Some(json!({
                "transformations": [],
                "mode": "each"
            })),
        );

        assert!(!node.validate());
    }

    #[test]
    fn test_add_field_transformation() {
        let node = TransformNode::new(
            Uuid::new_v4(),
            "Test Transform".to_string(),
            "transform".to_string(),
            None,
            Some(json!({
                "transformations": [{
                    "operation": "add_field",
                    "new_field": "processed",
                    "value": true
                }],
                "mode": "each"
            })),
        );

        let mut input = std::collections::HashMap::new();
        input.insert(Uuid::new_v4(), json!([
            {"name": "item1", "value": 1},
            {"name": "item2", "value": 2}
        ]));

        let result = node.execute(&input);
        assert!(result.is_ok());
        
        if let Ok(Value::Array(items)) = result {
            assert_eq!(items.len(), 2);
            for item in items {
                assert_eq!(item["processed"], true);
                assert!(item["name"].is_string());
                assert!(item["value"].is_number());
            }
        }
    }

    #[test]
    fn test_sort_transformation() {
        let node = TransformNode::new(
            Uuid::new_v4(),
            "Test Transform".to_string(),
            "transform".to_string(),
            None,
            Some(json!({
                "transformations": [{
                    "operation": "sort",
                    "sort_key": "value",
                    "sort_order": "desc"
                }],
                "mode": "batch"
            })),
        );

        let mut input = std::collections::HashMap::new();
        input.insert(Uuid::new_v4(), json!([
            {"name": "item1", "value": 1},
            {"name": "item3", "value": 3},
            {"name": "item2", "value": 2}
        ]));

        let result = node.execute(&input);
        assert!(result.is_ok());
        
        if let Ok(Value::Array(items)) = result {
            assert_eq!(items.len(), 3);
            assert_eq!(items[0]["value"], 3);
            assert_eq!(items[1]["value"], 2);
            assert_eq!(items[2]["value"], 1);
        }
    }
}