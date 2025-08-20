use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};

use crate::execution::NodeOutput;
use crate::node::{INode, NodeId, Error};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompareDatasetsNode {
    pub id: NodeId,
    pub name: String,
    pub description: Option<String>,
    pub parameters: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompareDatasetsParams {
    #[serde(default = "default_operation")]
    pub operation: String, // "difference", "intersection", "union", "equality", "contains"
    #[serde(default)]
    pub dataset1_field: Option<String>,
    #[serde(default)]
    pub dataset2_field: Option<String>,
    #[serde(default)]
    pub compare_fields: Vec<CompareField>,
    #[serde(default)]
    pub ignore_case: bool,
    #[serde(default)]
    pub fuzzy_compare: bool,
    #[serde(default = "default_similarity_threshold")]
    pub similarity_threshold: f64,
    #[serde(default = "default_output_format")]
    pub output_format: String, // "summary", "detailed", "counts_only"
    #[serde(default)]
    pub output_field: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompareField {
    pub field_name: String,
    #[serde(default)]
    pub ignore_case: Option<bool>,
    #[serde(default)]
    pub data_type: Option<String>, // "string", "number", "date", "boolean"
}

fn default_operation() -> String { "difference".to_string() }
fn default_similarity_threshold() -> f64 { 0.8 }
fn default_output_format() -> String { "detailed".to_string() }

impl Default for CompareDatasetsParams {
    fn default() -> Self {
        Self {
            operation: default_operation(),
            dataset1_field: None,
            dataset2_field: None,
            compare_fields: vec![],
            ignore_case: false,
            fuzzy_compare: false,
            similarity_threshold: default_similarity_threshold(),
            output_format: default_output_format(),
            output_field: None,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ComparisonResult {
    pub operation: String,
    pub dataset1_count: usize,
    pub dataset2_count: usize,
    pub matches: Vec<Value>,
    pub differences: Vec<Value>,
    pub unique_to_dataset1: Vec<Value>,
    pub unique_to_dataset2: Vec<Value>,
    pub summary: ComparisonSummary,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ComparisonSummary {
    pub total_matches: usize,
    pub total_differences: usize,
    pub unique_to_dataset1_count: usize,
    pub unique_to_dataset2_count: usize,
    pub similarity_percentage: f64,
}

impl INode for CompareDatasetsNode {
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
            serde_json::from_value::<CompareDatasetsParams>(params_value.clone())
                .unwrap_or_default()
        } else {
            CompareDatasetsParams::default()
        };

        let input_data: Vec<Value> = input.values().cloned().collect();
        
        if input_data.is_empty() {
            return Err(Error::NodeExecution("No input data provided".to_string()));
        }

        let item = &input_data[0];
        
        // Extract datasets
        let dataset1 = self.extract_dataset(item, &params.dataset1_field)?;
        let dataset2 = self.extract_dataset(item, &params.dataset2_field)?;

        // Perform comparison
        let result = self.compare_datasets(&dataset1, &dataset2, &params)?;

        // Format output
        let output = match params.output_format.as_str() {
            "summary" => json!({
                "summary": result.summary
            }),
            "counts_only" => json!({
                "dataset1_count": result.dataset1_count,
                "dataset2_count": result.dataset2_count,
                "matches_count": result.summary.total_matches,
                "differences_count": result.summary.total_differences,
                "unique_to_dataset1_count": result.summary.unique_to_dataset1_count,
                "unique_to_dataset2_count": result.summary.unique_to_dataset2_count,
                "similarity_percentage": result.summary.similarity_percentage
            }),
            _ => json!(result), // detailed
        };

        if let Some(output_field) = &params.output_field {
            let mut result_item = item.clone();
            if let Some(obj) = result_item.as_object_mut() {
                obj.insert(output_field.clone(), output);
            }
            Ok(result_item)
        } else {
            Ok(output)
        }
    }

    fn validate(&self) -> bool {
        if let Some(params_value) = &self.parameters {
            if let Ok(params) = serde_json::from_value::<CompareDatasetsParams>(params_value.clone()) {
                let valid_operations = ["difference", "intersection", "union", "equality", "contains"];
                let valid_formats = ["summary", "detailed", "counts_only"];
                
                return valid_operations.contains(&params.operation.as_str()) &&
                       valid_formats.contains(&params.output_format.as_str()) &&
                       params.similarity_threshold >= 0.0 && params.similarity_threshold <= 1.0;
            }
        }
        true
    }

    fn dependencies(&self) -> Vec<NodeId> {
        vec![]
    }
}

impl CompareDatasetsNode {
    fn extract_dataset(&self, item: &Value, field_name: &Option<String>) -> Result<Vec<Value>, Error> {
        let dataset = if let Some(field) = field_name {
            item.get(field).cloned().unwrap_or(Value::Array(vec![]))
        } else {
            item.clone()
        };

        match dataset {
            Value::Array(arr) => Ok(arr),
            Value::Object(_) => Ok(vec![dataset]),
            _ => Ok(vec![dataset]),
        }
    }

    fn compare_datasets(
        &self,
        dataset1: &[Value],
        dataset2: &[Value],
        params: &CompareDatasetsParams,
    ) -> Result<ComparisonResult, Error> {
        match params.operation.as_str() {
            "difference" => self.find_differences(dataset1, dataset2, params),
            "intersection" => self.find_intersection(dataset1, dataset2, params),
            "union" => self.find_union(dataset1, dataset2, params),
            "equality" => self.check_equality(dataset1, dataset2, params),
            "contains" => self.check_contains(dataset1, dataset2, params),
            _ => Err(Error::Validation(format!("Unsupported operation: {}", params.operation))),
        }
    }

    fn find_differences(
        &self,
        dataset1: &[Value],
        dataset2: &[Value],
        params: &CompareDatasetsParams,
    ) -> Result<ComparisonResult, Error> {
        let mut matches = Vec::new();
        let mut differences = Vec::new();
        let mut unique_to_dataset1 = Vec::new();
        let mut unique_to_dataset2 = Vec::new();

        // Create lookup for dataset2
        let dataset2_lookup = self.create_lookup(dataset2, params)?;

        for item1 in dataset1 {
            let key1 = self.generate_key(item1, params)?;
            if dataset2_lookup.contains_key(&key1) {
                let item2 = &dataset2_lookup[&key1];
                if self.items_equal(item1, item2, params)? {
                    matches.push(json!({
                        "dataset1_item": item1,
                        "dataset2_item": item2,
                        "match_type": "exact"
                    }));
                } else {
                    differences.push(json!({
                        "dataset1_item": item1,
                        "dataset2_item": item2,
                        "difference_type": "modified"
                    }));
                }
            } else {
                unique_to_dataset1.push(item1.clone());
            }
        }

        // Find items unique to dataset2
        let dataset1_lookup = self.create_lookup(dataset1, params)?;
        for item2 in dataset2 {
            let key2 = self.generate_key(item2, params)?;
            if !dataset1_lookup.contains_key(&key2) {
                unique_to_dataset2.push(item2.clone());
            }
        }

        let summary = self.calculate_summary(&matches, &differences, &unique_to_dataset1, &unique_to_dataset2, dataset1.len(), dataset2.len());

        Ok(ComparisonResult {
            operation: params.operation.clone(),
            dataset1_count: dataset1.len(),
            dataset2_count: dataset2.len(),
            matches,
            differences,
            unique_to_dataset1,
            unique_to_dataset2,
            summary,
        })
    }

    fn find_intersection(
        &self,
        dataset1: &[Value],
        dataset2: &[Value],
        params: &CompareDatasetsParams,
    ) -> Result<ComparisonResult, Error> {
        let mut matches = Vec::new();
        let dataset2_lookup = self.create_lookup(dataset2, params)?;

        for item1 in dataset1 {
            let key1 = self.generate_key(item1, params)?;
            if let Some(item2) = dataset2_lookup.get(&key1) {
                matches.push(json!({
                    "dataset1_item": item1,
                    "dataset2_item": item2,
                    "match_type": "intersection"
                }));
            }
        }

        let summary = self.calculate_summary(&matches, &[], &[], &[], dataset1.len(), dataset2.len());

        Ok(ComparisonResult {
            operation: params.operation.clone(),
            dataset1_count: dataset1.len(),
            dataset2_count: dataset2.len(),
            matches,
            differences: vec![],
            unique_to_dataset1: vec![],
            unique_to_dataset2: vec![],
            summary,
        })
    }

    fn find_union(
        &self,
        dataset1: &[Value],
        dataset2: &[Value],
        params: &CompareDatasetsParams,
    ) -> Result<ComparisonResult, Error> {
        let mut union_items = Vec::new();
        let mut seen_keys = HashSet::new();

        // Add all items from dataset1
        for item in dataset1 {
            let key = self.generate_key(item, params)?;
            if seen_keys.insert(key) {
                union_items.push(item.clone());
            }
        }

        // Add unique items from dataset2
        for item in dataset2 {
            let key = self.generate_key(item, params)?;
            if seen_keys.insert(key) {
                union_items.push(item.clone());
            }
        }

        let summary = ComparisonSummary {
            total_matches: union_items.len(),
            total_differences: 0,
            unique_to_dataset1_count: 0,
            unique_to_dataset2_count: 0,
            similarity_percentage: 100.0,
        };

        Ok(ComparisonResult {
            operation: params.operation.clone(),
            dataset1_count: dataset1.len(),
            dataset2_count: dataset2.len(),
            matches: union_items.into_iter().map(|item| json!({
                "item": item,
                "match_type": "union"
            })).collect(),
            differences: vec![],
            unique_to_dataset1: vec![],
            unique_to_dataset2: vec![],
            summary,
        })
    }

    fn check_equality(
        &self,
        dataset1: &[Value],
        dataset2: &[Value],
        params: &CompareDatasetsParams,
    ) -> Result<ComparisonResult, Error> {
        let equal = dataset1.len() == dataset2.len() && 
            dataset1.iter().all(|item1| {
                dataset2.iter().any(|item2| {
                    self.items_equal(item1, item2, params).unwrap_or(false)
                })
            });

        let similarity_percentage = if equal { 100.0 } else { 0.0 };

        let summary = ComparisonSummary {
            total_matches: if equal { dataset1.len() } else { 0 },
            total_differences: if equal { 0 } else { dataset1.len().max(dataset2.len()) },
            unique_to_dataset1_count: 0,
            unique_to_dataset2_count: 0,
            similarity_percentage,
        };

        Ok(ComparisonResult {
            operation: params.operation.clone(),
            dataset1_count: dataset1.len(),
            dataset2_count: dataset2.len(),
            matches: if equal { dataset1.iter().map(|item| json!({
                "item": item,
                "match_type": "equality"
            })).collect() } else { vec![] },
            differences: vec![],
            unique_to_dataset1: vec![],
            unique_to_dataset2: vec![],
            summary,
        })
    }

    fn check_contains(
        &self,
        dataset1: &[Value],
        dataset2: &[Value],
        params: &CompareDatasetsParams,
    ) -> Result<ComparisonResult, Error> {
        let dataset1_lookup = self.create_lookup(dataset1, params)?;
        let mut contained_items = Vec::new();
        let mut not_contained = Vec::new();

        for item2 in dataset2 {
            let key2 = self.generate_key(item2, params)?;
            if dataset1_lookup.contains_key(&key2) {
                contained_items.push(json!({
                    "item": item2,
                    "match_type": "contained"
                }));
            } else {
                not_contained.push(item2.clone());
            }
        }

        let similarity_percentage = if dataset2.is_empty() { 
            100.0 
        } else { 
            (contained_items.len() as f64 / dataset2.len() as f64) * 100.0 
        };

        let summary = ComparisonSummary {
            total_matches: contained_items.len(),
            total_differences: not_contained.len(),
            unique_to_dataset1_count: 0,
            unique_to_dataset2_count: not_contained.len(),
            similarity_percentage,
        };

        Ok(ComparisonResult {
            operation: params.operation.clone(),
            dataset1_count: dataset1.len(),
            dataset2_count: dataset2.len(),
            matches: contained_items,
            differences: vec![],
            unique_to_dataset1: vec![],
            unique_to_dataset2: not_contained,
            summary,
        })
    }

    fn create_lookup(&self, dataset: &[Value], params: &CompareDatasetsParams) -> Result<HashMap<String, Value>, Error> {
        let mut lookup = HashMap::new();
        for item in dataset {
            let key = self.generate_key(item, params)?;
            lookup.insert(key, item.clone());
        }
        Ok(lookup)
    }

    fn generate_key(&self, item: &Value, params: &CompareDatasetsParams) -> Result<String, Error> {
        if params.compare_fields.is_empty() {
            // Use entire item as key
            return Ok(item.to_string());
        }

        let mut key_parts = Vec::new();
        for field in &params.compare_fields {
            if let Some(value) = item.get(&field.field_name) {
                let mut value_str = value.to_string();
                if params.ignore_case || field.ignore_case.unwrap_or(false) {
                    value_str = value_str.to_lowercase();
                }
                key_parts.push(value_str);
            }
        }

        Ok(key_parts.join("|"))
    }

    fn items_equal(&self, item1: &Value, item2: &Value, params: &CompareDatasetsParams) -> Result<bool, Error> {
        if params.compare_fields.is_empty() {
            return Ok(item1 == item2);
        }

        for field in &params.compare_fields {
            let value1 = item1.get(&field.field_name);
            let value2 = item2.get(&field.field_name);

            match (value1, value2) {
                (Some(v1), Some(v2)) => {
                    if !self.values_equal(v1, v2, field, params)? {
                        return Ok(false);
                    }
                }
                (None, None) => continue,
                _ => return Ok(false),
            }
        }

        Ok(true)
    }

    fn values_equal(&self, value1: &Value, value2: &Value, field: &CompareField, params: &CompareDatasetsParams) -> Result<bool, Error> {
        if params.fuzzy_compare {
            return Ok(self.fuzzy_compare(value1, value2, params.similarity_threshold));
        }

        let ignore_case = params.ignore_case || field.ignore_case.unwrap_or(false);

        match (value1, value2) {
            (Value::String(s1), Value::String(s2)) => {
                if ignore_case {
                    Ok(s1.to_lowercase() == s2.to_lowercase())
                } else {
                    Ok(s1 == s2)
                }
            }
            _ => Ok(value1 == value2),
        }
    }

    fn fuzzy_compare(&self, value1: &Value, value2: &Value, threshold: f64) -> bool {
        let str1 = value1.to_string();
        let str2 = value2.to_string();
        
        if str1.is_empty() && str2.is_empty() {
            return true;
        }
        
        if str1.is_empty() || str2.is_empty() {
            return false;
        }

        let similarity = self.calculate_similarity(&str1, &str2);
        similarity >= threshold
    }

    fn calculate_similarity(&self, str1: &str, str2: &str) -> f64 {
        // Simple Levenshtein distance-based similarity
        let len1 = str1.chars().count();
        let len2 = str2.chars().count();
        
        if len1 == 0 && len2 == 0 {
            return 1.0;
        }
        
        let max_len = len1.max(len2);
        let distance = self.levenshtein_distance(str1, str2);
        
        1.0 - (distance as f64 / max_len as f64)
    }

    fn levenshtein_distance(&self, str1: &str, str2: &str) -> usize {
        let chars1: Vec<char> = str1.chars().collect();
        let chars2: Vec<char> = str2.chars().collect();
        let len1 = chars1.len();
        let len2 = chars2.len();

        let mut dp = vec![vec![0; len2 + 1]; len1 + 1];

        for i in 0..=len1 {
            dp[i][0] = i;
        }
        for j in 0..=len2 {
            dp[0][j] = j;
        }

        for i in 1..=len1 {
            for j in 1..=len2 {
                let cost = if chars1[i - 1] == chars2[j - 1] { 0 } else { 1 };
                dp[i][j] = (dp[i - 1][j] + 1)
                    .min(dp[i][j - 1] + 1)
                    .min(dp[i - 1][j - 1] + cost);
            }
        }

        dp[len1][len2]
    }

    fn calculate_summary(
        &self,
        matches: &[Value],
        differences: &[Value],
        unique_to_dataset1: &[Value],
        unique_to_dataset2: &[Value],
        dataset1_len: usize,
        dataset2_len: usize,
    ) -> ComparisonSummary {
        let total_items = dataset1_len.max(dataset2_len);
        let similarity_percentage = if total_items == 0 {
            100.0
        } else {
            (matches.len() as f64 / total_items as f64) * 100.0
        };

        ComparisonSummary {
            total_matches: matches.len(),
            total_differences: differences.len(),
            unique_to_dataset1_count: unique_to_dataset1.len(),
            unique_to_dataset2_count: unique_to_dataset2.len(),
            similarity_percentage,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_compare_datasets_node_creation() {
        let node = CompareDatasetsNode::new(
            uuid::Uuid::new_v4(),
            "Test Compare Datasets".to_string(),
            "compareDatasets".to_string(),
            Some("Test description".to_string()),
            Some(json!({
                "operation": "difference",
                "output_format": "detailed"
            })),
        );

        assert_eq!(node.name(), "Test Compare Datasets");
        assert!(node.validate());
    }

    #[test]
    fn test_find_differences() {
        let node = CompareDatasetsNode::new(
            uuid::Uuid::new_v4(),
            "Compare Datasets".to_string(),
            "compareDatasets".to_string(),
            None,
            Some(json!({
                "operation": "difference",
                "dataset1_field": "list1",
                "dataset2_field": "list2",
                "compare_fields": [{"field_name": "id"}]
            })),
        );

        let mut input = HashMap::new();
        input.insert(uuid::Uuid::new_v4(), json!({
            "list1": [
                {"id": 1, "name": "Alice"},
                {"id": 2, "name": "Bob"},
                {"id": 3, "name": "Charlie"}
            ],
            "list2": [
                {"id": 1, "name": "Alice"},
                {"id": 2, "name": "Robert"},
                {"id": 4, "name": "David"}
            ]
        }));

        let result = node.execute(&input);
        assert!(result.is_ok());

        if let Ok(output) = result {
            assert!(output.get("matches").is_some());
            assert!(output.get("differences").is_some());
            assert!(output.get("unique_to_dataset1").is_some());
            assert!(output.get("unique_to_dataset2").is_some());
            
            let summary = &output["summary"];
            assert!(summary.get("total_matches").is_some());
            assert!(summary.get("similarity_percentage").is_some());
        }
    }

    #[test]
    fn test_intersection() {
        let node = CompareDatasetsNode::new(
            uuid::Uuid::new_v4(),
            "Compare Datasets".to_string(),
            "compareDatasets".to_string(),
            None,
            Some(json!({
                "operation": "intersection",
                "dataset1_field": "list1",
                "dataset2_field": "list2",
                "compare_fields": [{"field_name": "id"}]
            })),
        );

        let mut input = HashMap::new();
        input.insert(uuid::Uuid::new_v4(), json!({
            "list1": [
                {"id": 1, "name": "Alice"},
                {"id": 2, "name": "Bob"}
            ],
            "list2": [
                {"id": 1, "name": "Alice"},
                {"id": 3, "name": "Charlie"}
            ]
        }));

        let result = node.execute(&input);
        assert!(result.is_ok());

        if let Ok(output) = result {
            let matches = output["matches"].as_array().unwrap();
            assert_eq!(matches.len(), 1); // Only id:1 should match
        }
    }

    #[test]
    fn test_equality() {
        let node = CompareDatasetsNode::new(
            uuid::Uuid::new_v4(),
            "Compare Datasets".to_string(),
            "compareDatasets".to_string(),
            None,
            Some(json!({
                "operation": "equality",
                "dataset1_field": "list1",
                "dataset2_field": "list2"
            })),
        );

        let mut input = HashMap::new();
        input.insert(uuid::Uuid::new_v4(), json!({
            "list1": [{"id": 1}, {"id": 2}],
            "list2": [{"id": 1}, {"id": 2}]
        }));

        let result = node.execute(&input);
        assert!(result.is_ok());

        if let Ok(output) = result {
            let summary = &output["summary"];
            assert_eq!(summary["similarity_percentage"], 100.0);
        }
    }

    #[test]
    fn test_fuzzy_compare() {
        let node = CompareDatasetsNode::new(
            uuid::Uuid::new_v4(),
            "Compare Datasets".to_string(),
            "compareDatasets".to_string(),
            None,
            Some(json!({
                "operation": "difference",
                "dataset1_field": "list1",
                "dataset2_field": "list2",
                "fuzzy_compare": true,
                "similarity_threshold": 0.8,
                "compare_fields": [{"field_name": "name"}]
            })),
        );

        let mut input = HashMap::new();
        input.insert(uuid::Uuid::new_v4(), json!({
            "list1": [{"name": "Alice"}],
            "list2": [{"name": "Alicia"}] // Similar but not exact
        }));

        let result = node.execute(&input);
        assert!(result.is_ok());
    }

    #[test]
    fn test_counts_only_output() {
        let node = CompareDatasetsNode::new(
            uuid::Uuid::new_v4(),
            "Compare Datasets".to_string(),
            "compareDatasets".to_string(),
            None,
            Some(json!({
                "operation": "difference",
                "dataset1_field": "list1",
                "dataset2_field": "list2",
                "output_format": "counts_only"
            })),
        );

        let mut input = HashMap::new();
        input.insert(uuid::Uuid::new_v4(), json!({
            "list1": [{"id": 1}, {"id": 2}],
            "list2": [{"id": 1}, {"id": 3}]
        }));

        let result = node.execute(&input);
        assert!(result.is_ok());

        if let Ok(output) = result {
            assert!(output.get("dataset1_count").is_some());
            assert!(output.get("dataset2_count").is_some());
            assert!(output.get("matches_count").is_some());
            assert!(output.get("similarity_percentage").is_some());
            
            // Should not have detailed arrays in counts_only mode
            assert!(output.get("matches").is_none());
            assert!(output.get("differences").is_none());
        }
    }

    #[test]
    fn test_validation() {
        let valid_node = CompareDatasetsNode::new(
            uuid::Uuid::new_v4(),
            "Valid".to_string(),
            "compareDatasets".to_string(),
            None,
            Some(json!({
                "operation": "difference",
                "output_format": "detailed",
                "similarity_threshold": 0.5
            })),
        );
        assert!(valid_node.validate());

        let invalid_operation_node = CompareDatasetsNode::new(
            uuid::Uuid::new_v4(),
            "Invalid Operation".to_string(),
            "compareDatasets".to_string(),
            None,
            Some(json!({
                "operation": "invalidOperation"
            })),
        );
        assert!(!invalid_operation_node.validate());

        let invalid_threshold_node = CompareDatasetsNode::new(
            uuid::Uuid::new_v4(),
            "Invalid Threshold".to_string(),
            "compareDatasets".to_string(),
            None,
            Some(json!({
                "similarity_threshold": 1.5
            })),
        );
        assert!(!invalid_threshold_node.validate());
    }
}