use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::execution::NodeOutput;
use crate::util;

pub mod code;
pub mod compare_datasets;
pub mod datetime;
pub mod execute_command;
pub mod file;
pub mod filter;
pub mod function;
pub mod http_request;
pub mod if_node;
pub mod json;
pub mod limit;
pub mod loop_node;
pub mod math;
pub mod merge;
pub mod move_binary_data;
pub mod openai;
pub mod remove_duplicates;
pub mod rename_keys;
pub mod schedule_trigger;
pub mod set;
pub mod sort;
pub mod split;
pub mod sticky_note;
pub mod switch;
pub mod text;
pub mod time;
pub mod transform;
pub mod trigger;
pub mod wait;
pub mod webhook;

pub type NodeId = Uuid;
pub type Error = util::Error;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Node {
    Code(code::CodeNode),
    CompareDatasets(compare_datasets::CompareDatasetsNode),
    DateTime(datetime::DateTimeNode),
    ExecuteCommand(execute_command::ExecuteCommandNode),
    File(file::FileNode),
    Filter(filter::FilterNode),
    Function(function::FunctionNode),
    HttpRequest(http_request::HttpRequestNode),
    If(if_node::IfNode),
    Json(json::JsonNode),
    Limit(limit::LimitNode),
    Loop(loop_node::LoopNode),
    Math(math::MathNode),
    Merge(merge::MergeNode),
    MoveBinaryData(move_binary_data::MoveBinaryDataNode),
    RemoveDuplicates(remove_duplicates::RemoveDuplicatesNode),
    RenameKeys(rename_keys::RenameKeysNode),
    ScheduleTrigger(schedule_trigger::ScheduleTriggerNode),
    Set(set::SetNode),
    Sort(sort::SortNode),
    Split(split::SplitNode),
    StickyNote(sticky_note::StickyNoteNode),
    Switch(switch::SwitchNode),
    Text(text::TextNode),
    Time(time::TimeNode),
    Transform(transform::TransformNode),
    Trigger(trigger::TriggerNode),
    Wait(wait::WaitNode),
    Webhook(webhook::WebhookNode),
    Openai(openai::OpenaiNode),
}

impl Node {
    pub fn from_json(node_id: NodeId, node_data: &Value) -> util::Result<Self> {
        let node_type = node_data.get("type").and_then(|v| v.as_str()).unwrap_or("");
        let node_name = node_data
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let node_description = node_data
            .get("description")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let parameters = node_data.get("parameters").cloned();

        let node = match node_type {
            "code" => Node::Code(code::CodeNode {
                id: node_id,
                name: node_name,
                description: node_description,
                parameters,
            }),
            "compareDatasets" | "compare_datasets" => Node::CompareDatasets(compare_datasets::CompareDatasetsNode {
                id: node_id,
                name: node_name,
                description: node_description,
                parameters,
            }),
            "dateTime" | "datetime" => Node::DateTime(datetime::DateTimeNode {
                id: node_id,
                name: node_name,
                description: node_description,
                parameters,
            }),
            "executeCommand" | "execute_command" => Node::ExecuteCommand(execute_command::ExecuteCommandNode {
                id: node_id,
                name: node_name,
                description: node_description,
                parameters,
            }),
            "file" => Node::File(file::FileNode {
                id: node_id,
                name: node_name,
                description: node_description,
                parameters,
            }),
            "filter" => Node::Filter(filter::FilterNode {
                id: node_id,
                name: node_name,
                description: node_description,
                parameters,
            }),
            "function" => Node::Function(function::FunctionNode {
                id: node_id,
                name: node_name,
                description: node_description,
                parameters,
            }),
            "http_request" | "httpRequest" => Node::HttpRequest(http_request::HttpRequestNode {
                id: node_id,
                name: node_name,
                description: node_description,
                parameters,
            }),
            "if" => Node::If(if_node::IfNode {
                id: node_id,
                name: node_name,
                description: node_description,
                parameters,
            }),
            "json" => Node::Json(json::JsonNode {
                id: node_id,
                name: node_name,
                description: node_description,
                parameters,
            }),
            "limit" => Node::Limit(limit::LimitNode {
                id: node_id,
                name: node_name,
                description: node_description,
                parameters,
            }),
            "loop" => Node::Loop(loop_node::LoopNode {
                id: node_id,
                name: node_name,
                description: node_description,
                parameters,
            }),
            "math" => Node::Math(math::MathNode {
                id: node_id,
                name: node_name,
                description: node_description,
                parameters,
            }),
            "merge" => Node::Merge(merge::MergeNode {
                id: node_id,
                name: node_name,
                description: node_description,
                parameters,
            }),
            "moveBinaryData" | "move_binary_data" => Node::MoveBinaryData(move_binary_data::MoveBinaryDataNode {
                id: node_id,
                name: node_name,
                description: node_description,
                parameters,
            }),
            "removeDuplicates" | "remove_duplicates" => Node::RemoveDuplicates(remove_duplicates::RemoveDuplicatesNode {
                id: node_id,
                name: node_name,
                description: node_description,
                parameters,
            }),
            "renameKeys" | "rename_keys" => Node::RenameKeys(rename_keys::RenameKeysNode {
                id: node_id,
                name: node_name,
                description: node_description,
                parameters,
            }),
            "scheduleTrigger" | "schedule_trigger" => Node::ScheduleTrigger(schedule_trigger::ScheduleTriggerNode {
                id: node_id,
                name: node_name,
                description: node_description,
                parameters,
            }),
            "set" => Node::Set(set::SetNode {
                id: node_id,
                name: node_name,
                description: node_description,
                parameters,
            }),
            "sort" => Node::Sort(sort::SortNode {
                id: node_id,
                name: node_name,
                description: node_description,
                parameters,
            }),
            "split" | "splitInBatches" | "splitOut" => Node::Split(split::SplitNode {
                id: node_id,
                name: node_name,
                description: node_description,
                parameters,
            }),
            "stickyNote" | "sticky_note" => Node::StickyNote(sticky_note::StickyNoteNode {
                id: node_id,
                name: node_name,
                description: node_description,
                parameters,
            }),
            "switch" => Node::Switch(switch::SwitchNode {
                id: node_id,
                name: node_name,
                description: node_description,
                parameters,
            }),
            "text" => Node::Text(text::TextNode {
                id: node_id,
                name: node_name,
                description: node_description,
                parameters,
            }),
            "time" => Node::Time(time::TimeNode {
                id: node_id,
                name: node_name,
                description: node_description,
                parameters,
            }),
            "transform" => Node::Transform(transform::TransformNode {
                id: node_id,
                name: node_name,
                description: node_description,
                parameters,
            }),
            "trigger" => Node::Trigger(trigger::TriggerNode {
                id: node_id,
                name: node_name,
                description: node_description,
                parameters,
            }),
            "wait" | "delay" => Node::Wait(wait::WaitNode {
                id: node_id,
                name: node_name,
                description: node_description,
                parameters,
            }),
            "webhook" => Node::Webhook(webhook::WebhookNode {
                id: node_id,
                name: node_name,
                description: node_description,
                parameters,
            }),
            "openai" => Node::Openai(openai::OpenaiNode {
                id: node_id,
                name: node_name,
                description: node_description,
                parameters,
            }),
            _ => {
                return Err(Error::Configuration(format!(
                    "Unknown node type: {}",
                    node_type
                )));
            }
        };

        Ok(node)
    }

    /// Parse nodes from JSON
    pub fn parse_nodes_from_json(nodes_obj: &Value) -> util::Result<HashMap<NodeId, Self>> {
        let mut nodes = HashMap::new();

        for (node_id_str, node_data) in nodes_obj.as_object().unwrap() {
            let node_id = Uuid::parse_str(node_id_str)?;
            let node = Node::from_json(node_id, node_data)?;
            nodes.insert(node_id, node);
        }

        Ok(nodes)
    }
}

pub trait INode {
    fn new(
        id: NodeId,
        name: String,
        node_type: String,
        description: Option<String>,
        parameters: Option<Value>,
    ) -> Self;
    fn id(&self) -> NodeId;
    fn name(&self) -> String;
    fn description(&self) -> Option<String>;
    fn parameter(&self) -> Option<Value>;
    fn execute(&self, input: &NodeOutput) -> Result<Value, Error>;
    // true: validation passed, false: validation failed
    fn validate(&self) -> bool;
    fn dependencies(&self) -> Vec<NodeId>;
}

impl INode for Node {
    fn new(
        id: NodeId,
        name: String,
        node_type: String,
        description: Option<String>,
        parameters: Option<Value>,
    ) -> Self {
        match node_type.as_str() {
            "code" => Node::Code(code::CodeNode::new(
                id,
                name,
                node_type,
                description,
                parameters,
            )),
            "compareDatasets" | "compare_datasets" => Node::CompareDatasets(compare_datasets::CompareDatasetsNode::new(
                id,
                name,
                node_type,
                description,
                parameters,
            )),
            "dateTime" | "datetime" => Node::DateTime(datetime::DateTimeNode::new(
                id,
                name,
                node_type,
                description,
                parameters,
            )),
            "executeCommand" | "execute_command" => Node::ExecuteCommand(execute_command::ExecuteCommandNode::new(
                id,
                name,
                node_type,
                description,
                parameters,
            )),
            "file" => Node::File(file::FileNode::new(
                id,
                name,
                node_type,
                description,
                parameters,
            )),
            "filter" => Node::Filter(filter::FilterNode::new(
                id,
                name,
                node_type,
                description,
                parameters,
            )),
            "function" => Node::Function(function::FunctionNode::new(
                id,
                name,
                node_type,
                description,
                parameters,
            )),
            "http_request" | "httpRequest" => Node::HttpRequest(http_request::HttpRequestNode::new(
                id,
                name,
                node_type,
                description,
                parameters,
            )),
            "if" => Node::If(if_node::IfNode::new(
                id,
                name,
                node_type,
                description,
                parameters,
            )),
            "json" => Node::Json(json::JsonNode::new(
                id,
                name,
                node_type,
                description,
                parameters,
            )),
            "limit" => Node::Limit(limit::LimitNode::new(
                id,
                name,
                node_type,
                description,
                parameters,
            )),
            "loop" => Node::Loop(loop_node::LoopNode::new(
                id,
                name,
                node_type,
                description,
                parameters,
            )),
            "math" => Node::Math(math::MathNode::new(
                id,
                name,
                node_type,
                description,
                parameters,
            )),
            "merge" => Node::Merge(merge::MergeNode::new(
                id,
                name,
                node_type,
                description,
                parameters,
            )),
            "moveBinaryData" | "move_binary_data" => Node::MoveBinaryData(move_binary_data::MoveBinaryDataNode::new(
                id,
                name,
                node_type,
                description,
                parameters,
            )),
            "removeDuplicates" | "remove_duplicates" => Node::RemoveDuplicates(remove_duplicates::RemoveDuplicatesNode::new(
                id,
                name,
                node_type,
                description,
                parameters,
            )),
            "renameKeys" | "rename_keys" => Node::RenameKeys(rename_keys::RenameKeysNode::new(
                id,
                name,
                node_type,
                description,
                parameters,
            )),
            "scheduleTrigger" | "schedule_trigger" => Node::ScheduleTrigger(schedule_trigger::ScheduleTriggerNode::new(
                id,
                name,
                node_type,
                description,
                parameters,
            )),
            "set" => Node::Set(set::SetNode::new(
                id,
                name,
                node_type,
                description,
                parameters,
            )),
            "sort" => Node::Sort(sort::SortNode::new(
                id,
                name,
                node_type,
                description,
                parameters,
            )),
            "split" | "splitInBatches" | "splitOut" => Node::Split(split::SplitNode::new(
                id,
                name,
                node_type,
                description,
                parameters,
            )),
            "stickyNote" | "sticky_note" => Node::StickyNote(sticky_note::StickyNoteNode::new(
                id,
                name,
                node_type,
                description,
                parameters,
            )),
            "switch" => Node::Switch(switch::SwitchNode::new(
                id,
                name,
                node_type,
                description,
                parameters,
            )),
            "text" => Node::Text(text::TextNode::new(
                id,
                name,
                node_type,
                description,
                parameters,
            )),
            "time" => Node::Time(time::TimeNode::new(
                id,
                name,
                node_type,
                description,
                parameters,
            )),
            "transform" => Node::Transform(transform::TransformNode::new(
                id,
                name,
                node_type,
                description,
                parameters,
            )),
            "trigger" => Node::Trigger(trigger::TriggerNode::new(
                id,
                name,
                node_type,
                description,
                parameters,
            )),
            "wait" | "delay" => Node::Wait(wait::WaitNode::new(
                id,
                name,
                node_type,
                description,
                parameters,
            )),
            "webhook" => Node::Webhook(webhook::WebhookNode::new(
                id,
                name,
                node_type,
                description,
                parameters,
            )),
            "openai" => Node::Openai(openai::OpenaiNode::new(
                id,
                name,
                node_type,
                description,
                parameters,
            )),
            _ => panic!("Unknown node type: {}", node_type),
        }
    }

    fn id(&self) -> NodeId {
        match self {
            Node::Code(node) => node.id(),
            Node::CompareDatasets(node) => node.id(),
            Node::DateTime(node) => node.id(),
            Node::ExecuteCommand(node) => node.id(),
            Node::File(node) => node.id(),
            Node::Filter(node) => node.id(),
            Node::Function(node) => node.id(),
            Node::HttpRequest(node) => node.id(),
            Node::If(node) => node.id(),
            Node::Json(node) => node.id(),
            Node::Limit(node) => node.id(),
            Node::Loop(node) => node.id(),
            Node::Math(node) => node.id(),
            Node::Merge(node) => node.id(),
            Node::MoveBinaryData(node) => node.id(),
            Node::RemoveDuplicates(node) => node.id(),
            Node::RenameKeys(node) => node.id(),
            Node::ScheduleTrigger(node) => node.id(),
            Node::Set(node) => node.id(),
            Node::Sort(node) => node.id(),
            Node::Split(node) => node.id(),
            Node::StickyNote(node) => node.id(),
            Node::Switch(node) => node.id(),
            Node::Text(node) => node.id(),
            Node::Time(node) => node.id(),
            Node::Transform(node) => node.id(),
            Node::Trigger(node) => node.id(),
            Node::Wait(node) => node.id(),
            Node::Webhook(node) => node.id(),
            Node::Openai(node) => node.id(),
        }
    }

    fn name(&self) -> String {
        match self {
            Node::Code(node) => node.name(),
            Node::CompareDatasets(node) => node.name(),
            Node::DateTime(node) => node.name(),
            Node::ExecuteCommand(node) => node.name(),
            Node::File(node) => node.name(),
            Node::Filter(node) => node.name(),
            Node::Function(node) => node.name(),
            Node::HttpRequest(node) => node.name(),
            Node::If(node) => node.name(),
            Node::Json(node) => node.name(),
            Node::Limit(node) => node.name(),
            Node::Loop(node) => node.name(),
            Node::Math(node) => node.name(),
            Node::Merge(node) => node.name(),
            Node::MoveBinaryData(node) => node.name(),
            Node::RemoveDuplicates(node) => node.name(),
            Node::RenameKeys(node) => node.name(),
            Node::ScheduleTrigger(node) => node.name(),
            Node::Set(node) => node.name(),
            Node::Sort(node) => node.name(),
            Node::Split(node) => node.name(),
            Node::StickyNote(node) => node.name(),
            Node::Switch(node) => node.name(),
            Node::Text(node) => node.name(),
            Node::Time(node) => node.name(),
            Node::Transform(node) => node.name(),
            Node::Trigger(node) => node.name(),
            Node::Wait(node) => node.name(),
            Node::Webhook(node) => node.name(),
            Node::Openai(node) => node.name(),
        }
    }

    fn description(&self) -> Option<String> {
        match self {
            Node::Code(node) => node.description(),
            Node::CompareDatasets(node) => node.description(),
            Node::DateTime(node) => node.description(),
            Node::ExecuteCommand(node) => node.description(),
            Node::File(node) => node.description(),
            Node::Filter(node) => node.description(),
            Node::Function(node) => node.description(),
            Node::HttpRequest(node) => node.description(),
            Node::If(node) => node.description(),
            Node::Json(node) => node.description(),
            Node::Limit(node) => node.description(),
            Node::Loop(node) => node.description(),
            Node::Math(node) => node.description(),
            Node::Merge(node) => node.description(),
            Node::MoveBinaryData(node) => node.description(),
            Node::RemoveDuplicates(node) => node.description(),
            Node::RenameKeys(node) => node.description(),
            Node::ScheduleTrigger(node) => node.description(),
            Node::Set(node) => node.description(),
            Node::Sort(node) => node.description(),
            Node::Split(node) => node.description(),
            Node::StickyNote(node) => node.description(),
            Node::Switch(node) => node.description(),
            Node::Text(node) => node.description(),
            Node::Time(node) => node.description(),
            Node::Transform(node) => node.description(),
            Node::Trigger(node) => node.description(),
            Node::Wait(node) => node.description(),
            Node::Webhook(node) => node.description(),
            Node::Openai(node) => node.description(),
        }
    }

    fn parameter(&self) -> Option<Value> {
        match self {
            Node::Code(node) => node.parameter(),
            Node::CompareDatasets(node) => node.parameter(),
            Node::DateTime(node) => node.parameter(),
            Node::ExecuteCommand(node) => node.parameter(),
            Node::File(node) => node.parameter(),
            Node::Filter(node) => node.parameter(),
            Node::Function(node) => node.parameter(),
            Node::HttpRequest(node) => node.parameter(),
            Node::If(node) => node.parameter(),
            Node::Json(node) => node.parameter(),
            Node::Limit(node) => node.parameter(),
            Node::Loop(node) => node.parameter(),
            Node::Math(node) => node.parameter(),
            Node::Merge(node) => node.parameter(),
            Node::MoveBinaryData(node) => node.parameter(),
            Node::RemoveDuplicates(node) => node.parameter(),
            Node::RenameKeys(node) => node.parameter(),
            Node::ScheduleTrigger(node) => node.parameter(),
            Node::Set(node) => node.parameter(),
            Node::Sort(node) => node.parameter(),
            Node::Split(node) => node.parameter(),
            Node::StickyNote(node) => node.parameter(),
            Node::Switch(node) => node.parameter(),
            Node::Text(node) => node.parameter(),
            Node::Time(node) => node.parameter(),
            Node::Transform(node) => node.parameter(),
            Node::Trigger(node) => node.parameter(),
            Node::Wait(node) => node.parameter(),
            Node::Webhook(node) => node.parameter(),
            Node::Openai(node) => node.parameter(),
        }
    }

    fn execute(&self, input: &NodeOutput) -> Result<Value, Error> {
        match self {
            Node::Code(node) => node.execute(input),
            Node::CompareDatasets(node) => node.execute(input),
            Node::DateTime(node) => node.execute(input),
            Node::ExecuteCommand(node) => node.execute(input),
            Node::File(node) => node.execute(input),
            Node::Filter(node) => node.execute(input),
            Node::Function(node) => node.execute(input),
            Node::HttpRequest(node) => node.execute(input),
            Node::If(node) => node.execute(input),
            Node::Json(node) => node.execute(input),
            Node::Limit(node) => node.execute(input),
            Node::Loop(node) => node.execute(input),
            Node::Math(node) => node.execute(input),
            Node::Merge(node) => node.execute(input),
            Node::MoveBinaryData(node) => node.execute(input),
            Node::RemoveDuplicates(node) => node.execute(input),
            Node::RenameKeys(node) => node.execute(input),
            Node::ScheduleTrigger(node) => node.execute(input),
            Node::Set(node) => node.execute(input),
            Node::Sort(node) => node.execute(input),
            Node::Split(node) => node.execute(input),
            Node::StickyNote(node) => node.execute(input),
            Node::Switch(node) => node.execute(input),
            Node::Text(node) => node.execute(input),
            Node::Time(node) => node.execute(input),
            Node::Transform(node) => node.execute(input),
            Node::Trigger(node) => node.execute(input),
            Node::Wait(node) => node.execute(input),
            Node::Webhook(node) => node.execute(input),
            Node::Openai(node) => node.execute(input),
        }
    }

    fn validate(&self) -> bool {
        match self {
            Node::Code(node) => node.validate(),
            Node::CompareDatasets(node) => node.validate(),
            Node::DateTime(node) => node.validate(),
            Node::ExecuteCommand(node) => node.validate(),
            Node::File(node) => node.validate(),
            Node::Filter(node) => node.validate(),
            Node::Function(node) => node.validate(),
            Node::HttpRequest(node) => node.validate(),
            Node::If(node) => node.validate(),
            Node::Json(node) => node.validate(),
            Node::Limit(node) => node.validate(),
            Node::Loop(node) => node.validate(),
            Node::Math(node) => node.validate(),
            Node::Merge(node) => node.validate(),
            Node::MoveBinaryData(node) => node.validate(),
            Node::RemoveDuplicates(node) => node.validate(),
            Node::RenameKeys(node) => node.validate(),
            Node::ScheduleTrigger(node) => node.validate(),
            Node::Set(node) => node.validate(),
            Node::Sort(node) => node.validate(),
            Node::Split(node) => node.validate(),
            Node::StickyNote(node) => node.validate(),
            Node::Switch(node) => node.validate(),
            Node::Text(node) => node.validate(),
            Node::Time(node) => node.validate(),
            Node::Transform(node) => node.validate(),
            Node::Trigger(node) => node.validate(),
            Node::Wait(node) => node.validate(),
            Node::Webhook(node) => node.validate(),
            Node::Openai(node) => node.validate(),
        }
    }

    fn dependencies(&self) -> Vec<NodeId> {
        match self {
            Node::Code(node) => node.dependencies(),
            Node::CompareDatasets(node) => node.dependencies(),
            Node::DateTime(node) => node.dependencies(),
            Node::ExecuteCommand(node) => node.dependencies(),
            Node::File(node) => node.dependencies(),
            Node::Filter(node) => node.dependencies(),
            Node::Function(node) => node.dependencies(),
            Node::HttpRequest(node) => node.dependencies(),
            Node::If(node) => node.dependencies(),
            Node::Json(node) => node.dependencies(),
            Node::Limit(node) => node.dependencies(),
            Node::Loop(node) => node.dependencies(),
            Node::Math(node) => node.dependencies(),
            Node::Merge(node) => node.dependencies(),
            Node::MoveBinaryData(node) => node.dependencies(),
            Node::RemoveDuplicates(node) => node.dependencies(),
            Node::RenameKeys(node) => node.dependencies(),
            Node::ScheduleTrigger(node) => node.dependencies(),
            Node::Set(node) => node.dependencies(),
            Node::Sort(node) => node.dependencies(),
            Node::Split(node) => node.dependencies(),
            Node::StickyNote(node) => node.dependencies(),
            Node::Switch(node) => node.dependencies(),
            Node::Text(node) => node.dependencies(),
            Node::Time(node) => node.dependencies(),
            Node::Transform(node) => node.dependencies(),
            Node::Trigger(node) => node.dependencies(),
            Node::Wait(node) => node.dependencies(),
            Node::Webhook(node) => node.dependencies(),
            Node::Openai(node) => node.dependencies(),
        }
    }
}
