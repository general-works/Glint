# Glint

[![Rust](https://img.shields.io/badge/built%20with-Rust-red)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Version](https://img.shields.io/badge/version-0.1.0-green.svg)](Cargo.toml)

A high-performance, type-safe workflow automation engine built in Rust. Glint empowers developers to create, execute, and manage complex workflows with precision and reliability.

## 🚀 Features

### Core Capabilities
- **JSON-Based Workflow Definition**: Define workflows using intuitive JSON configuration
- **Node-Based Architecture**: Modular node system supporting triggers, actions, and data transformations
- **Expression Engine**: Powerful expression system with node data references (`=${node-id}.field`)
- **Parallel Execution**: Intelligent batch processing for optimal performance
- **Cycle Detection**: Built-in circular dependency prevention
- **Type Safety**: Leveraging Rust's type system for robust workflow execution

### Comprehensive Node Library (25+ Node Types)

#### 🔧 Core System Nodes
- **Code**: Execute JavaScript code snippets for custom logic
- **ExecuteCommand**: Run shell commands and capture output 
- **Function**: Advanced JavaScript function execution with complex parameter handling
- **Set**: Data transformation and variable assignment with expression support
- **Transform**: Advanced data transformation operations
- **Webhook**: Handle incoming HTTP requests and send webhook notifications

#### 🔄 Control Flow Nodes
- **If**: Conditional branching based on expression evaluation
- **Loop**: Iteration control (for, while, forEach loops)  
- **Split**: Split data into batches or separate fields
- **Switch**: Multi-way branching based on expression values
- **Trigger**: Manual triggers and event-based workflow initiators
- **Wait**: Delay execution or wait until specific time

#### 📊 Data Processing Nodes
- **Filter**: Filter arrays based on conditions
- **Limit**: Limit number of items and implement pagination  
- **Math**: Mathematical operations (arithmetic, statistics, array functions)
- **Merge**: Combine data from multiple sources (append, zip, merge by key)
- **RemoveDuplicates**: Remove duplicate items with flexible comparison strategies
- **RenameKeys**: Rename object keys with regex support
- **Sort**: Sort data with multiple keys, custom comparison, and randomization
- **Text**: String manipulation (length, case conversion, regex, split/join)

#### ⏰ Time & Date Nodes  
- **DateTime**: Date/time operations, formatting, parsing, and arithmetic
- **Time**: Current time retrieval and temporal calculations

#### 🗂️ File & Data Nodes
- **File**: File system operations (read, write, create directories, file info)
- **Json**: JSON parsing, validation, extraction, and manipulation

#### 🌐 Integration Nodes
- **HttpRequest**: HTTP client for REST API calls
- **OpenAI**: GPT integration for AI-powered workflows

### Advanced Features
- **Execution Plans**: Automatic dependency resolution and batch optimization
- **Validation Engine**: Comprehensive workflow and node validation
- **Error Handling**: Robust error management and reporting
- **Real-time Status Tracking**: Monitor workflow execution progress
- **Binary Data Support**: Handle complex data types and file operations

## 📋 Quick Start

### Prerequisites
- Rust 1.70+ (2024 edition)
- Cargo package manager

### Installation

```bash
git clone https://github.com/your-username/glint.git
cd glint
cargo build --release
```

### Basic Usage

1. **Define a workflow** in JSON format:

```json
{
    "nodes": {
        "trigger-1": {
            "type": "trigger",
            "name": "Start Workflow",
            "description": "Manual trigger to start the workflow"
        },
        "process-1": {
            "type": "set",
            "name": "Process Data",
            "description": "Transform input data",
            "parameters": {
                "user": "John Doe",
                "email": "john@example.com"
            }
        }
    },
    "connections": {
        "trigger-1": ["process-1"]
    }
}
```

2. **Execute workflows**:

```bash
cargo run --bin glint
```

3. **Run examples**:

```bash
# Set operation example
cargo run --bin set_example

# Time processing example
cargo run --bin time_example

# OpenAI integration example  
cargo run --bin openai_example
```

## 🏗️ Architecture

### Node System
Glint uses a trait-based node architecture where each node implements the `INode` trait:

```rust
pub trait INode {
    fn id(&self) -> NodeId;
    fn name(&self) -> String;
    fn description(&self) -> Option<String>;
    fn parameter(&self) -> Option<Value>;
    fn validate(&self) -> bool;
    fn execute(&self, input: &NodeOutput) -> Result<Value, Error>;
    fn dependencies(&self) -> Vec<NodeId>;
}
```

### Execution Engine
- **Plan Generation**: Automatically generates execution plans with optimal batching
- **Dependency Resolution**: Ensures nodes execute in correct order
- **Parallel Processing**: Executes independent nodes concurrently
- **Status Tracking**: Real-time monitoring of execution progress

### Expression System
Powerful expression syntax for dynamic data access:
- `=${node-id}.field` - Reference data from other nodes
- Nested object and array access support
- Type-safe expression evaluation

## 🔧 Node Reference

### Text Processing
```json
{
    "type": "text",
    "parameters": {
        "operation": "replace",
        "input_field": "message",
        "search_value": "hello",
        "replace_value": "hi",
        "output_field": "result"
    }
}
```

### Mathematical Operations
```json
{
    "type": "math", 
    "parameters": {
        "operation": "add",
        "operand1_field": "price",
        "operand2_value": 10,
        "output_field": "total"
    }
}
```

### File Operations
```json
{
    "type": "file",
    "parameters": {
        "operation": "read_file",
        "file_path": "/path/to/file.txt",
        "output_field": "content"
    }
}
```

### Command Execution
```json
{
    "type": "executeCommand",
    "parameters": {
        "command": "echo 'Hello World'",
        "execute_once": true,
        "working_directory": "/tmp"
    }
}
```

### Data Sorting
```json
{
    "type": "sort",
    "parameters": {
        "sort_type": "multi",
        "sort_keys": [
            {"key": "name", "direction": "asc"},
            {"key": "age", "direction": "desc"}
        ]
    }
}
```

### Duplicate Removal
```json
{
    "type": "removeDuplicates",
    "parameters": {
        "compare": "selectedFields",
        "fields_to_compare": [
            {"field_name": "email"},
            {"field_name": "user_id"}
        ]
    }
}
```

### Key Renaming with Regex
```json
{
    "type": "renameKeys",
    "parameters": {
        "regex_replacements": [
            {"search_regex": "old(.*)", "replace_regex": "new$1"}
        ],
        "regex_case_insensitive": false
    }
}
```

## 📊 Glint vs n8n Comparison

| Feature | Glint | n8n |
|---------|-------|-----|
| **Runtime** | Rust (compiled, high-performance) | Node.js (interpreted) |
| **Type Safety** | Compile-time type checking | Runtime type validation |
| **Performance** | Optimized for speed and memory efficiency | Good for I/O-heavy workflows |
| **Deployment** | Single binary, minimal footprint | Requires Node.js runtime |
| **Workflow Definition** | JSON-based, programmatic | Visual editor + JSON |
| **Expression Engine** | Custom Rust implementation | JavaScript-based |
| **Memory Usage** | Low memory footprint | Higher memory usage |
| **Startup Time** | Near-instantaneous | Moderate startup time |
| **Node Types** | 25+ built-in nodes | 200+ pre-built integrations |
| **Execution Model** | Batch-optimized parallel execution | Sequential execution |
| **Error Handling** | Rust's Result type system | JavaScript exception handling |

### When to Choose Glint
- ✅ High-performance requirements
- ✅ Resource-constrained environments  
- ✅ Type safety is critical
- ✅ Programmatic workflow creation
- ✅ Embedded or edge deployments
- ✅ Complex data processing pipelines

### When to Choose n8n
- ✅ Visual workflow design preferred
- ✅ Extensive pre-built integrations needed
- ✅ Rapid prototyping
- ✅ Non-technical users
- ✅ Cloud-first deployment

## 🧪 Examples

### Complex Data Processing Pipeline
```json
{
    "nodes": {
        "trigger": {"type": "trigger", "name": "Start Processing"},
        "load-data": {
            "type": "file",
            "name": "Load CSV Data", 
            "parameters": {
                "operation": "read_file",
                "file_path": "data.csv"
            }
        },
        "parse-json": {
            "type": "json",
            "name": "Parse JSON",
            "parameters": {
                "operation": "parse",
                "input_field": "content" 
            }
        },
        "filter-active": {
            "type": "filter",
            "name": "Filter Active Users",
            "parameters": {
                "conditions": [{"field": "status", "operation": "equal", "value": "active"}]
            }
        },
        "remove-dupes": {
            "type": "removeDuplicates",
            "name": "Remove Duplicate Emails",
            "parameters": {
                "compare": "selectedFields",
                "fields_to_compare": [{"field_name": "email"}]
            }
        },
        "sort-users": {
            "type": "sort", 
            "name": "Sort by Registration Date",
            "parameters": {
                "sort_keys": [{"key": "registered_at", "direction": "desc"}]
            }
        },
        "limit-results": {
            "type": "limit",
            "name": "Take Top 100",
            "parameters": {"limit": 100}
        }
    },
    "connections": {
        "trigger": ["load-data"],
        "load-data": ["parse-json"],
        "parse-json": ["filter-active"], 
        "filter-active": ["remove-dupes"],
        "remove-dupes": ["sort-users"],
        "sort-users": ["limit-results"]
    }
}
```

### AI-Powered Content Processing
```json
{
    "nodes": {
        "trigger": {"type": "trigger", "name": "Process Content"},
        "input-text": {
            "type": "set",
            "parameters": {"content": "Raw article text here..."}
        },
        "clean-text": {
            "type": "text",
            "name": "Clean Text",
            "parameters": {
                "operation": "regex",
                "input_field": "content",
                "pattern": "[^a-zA-Z0-9\\s]",
                "replacement": ""
            }
        },
        "ai-summary": {
            "type": "openai",
            "name": "Generate Summary", 
            "parameters": {
                "model": "gpt-4",
                "messages": [{
                    "role": "user",
                    "content": "Summarize this text: =${clean-text}.result"
                }]
            }
        },
        "ai-keywords": {
            "type": "openai",
            "name": "Extract Keywords",
            "parameters": {
                "model": "gpt-4", 
                "messages": [{
                    "role": "user",
                    "content": "Extract key topics from: =${clean-text}.result"
                }]
            }
        },
        "combine-results": {
            "type": "set",
            "name": "Combine Results",
            "parameters": {
                "original": "=${input-text}.content",
                "cleaned": "=${clean-text}.result",
                "summary": "=${ai-summary}.response",
                "keywords": "=${ai-keywords}.response"
            }
        }
    },
    "connections": {
        "trigger": ["input-text"],
        "input-text": ["clean-text"],
        "clean-text": ["ai-summary", "ai-keywords"],
        "ai-summary": ["combine-results"],
        "ai-keywords": ["combine-results"]
    }
}
```

## 🧪 Testing

Run the comprehensive test suite:

```bash
# Run all tests (197 tests)
cargo test

# Run with output
cargo test -- --nocapture

# Run specific node tests
cargo test text::tests
cargo test math::tests  
cargo test execute_command::tests
```

## 🛠️ Development

### Project Structure
```
src/
├── lib.rs              # Library entry point
├── main.rs             # Main executable  
├── workflow/           # Workflow engine core
├── execution/          # Execution engine
├── node/               # Node implementations
│   ├── code.rs         # JavaScript code execution
│   ├── datetime.rs     # Date/time operations
│   ├── execute_command.rs # Shell command execution
│   ├── file.rs         # File system operations
│   ├── filter.rs       # Array filtering
│   ├── function.rs     # Advanced function execution
│   ├── http_request.rs # HTTP client
│   ├── if_node.rs      # Conditional logic
│   ├── json.rs         # JSON manipulation
│   ├── limit.rs        # Data limiting/pagination
│   ├── loop_node.rs    # Loop control structures
│   ├── math.rs         # Mathematical operations
│   ├── merge.rs        # Data merging
│   ├── openai.rs       # AI/GPT integration
│   ├── remove_duplicates.rs # Deduplication
│   ├── rename_keys.rs  # Key renaming
│   ├── set.rs          # Data assignment
│   ├── sort.rs         # Data sorting
│   ├── split.rs        # Data splitting
│   ├── switch.rs       # Multi-way branching
│   ├── text.rs         # String processing
│   ├── time.rs         # Time utilities
│   ├── transform.rs    # Data transformation
│   ├── trigger.rs      # Workflow triggers
│   ├── wait.rs         # Delay/scheduling
│   └── webhook.rs      # HTTP webhooks
├── expression/         # Expression parser
├── trigger/            # Trigger system
├── util/               # Utilities and error handling
└── examples/           # Example workflows
    ├── set/            # Data transformation examples
    ├── time/           # Time processing examples
    └── openai/         # AI integration examples
```

### Adding Custom Nodes

1. **Implement the INode trait**:
```rust
impl INode for CustomNode {
    fn execute(&self, input: &NodeOutput) -> Result<Value, Error> {
        // Your custom logic here
    }
}
```

2. **Register in the node factory**:
```rust
match node_type.as_str() {
    "custom" => Node::Custom(CustomNode::new(/* params */)),
    // ...
}
```

3. **Add comprehensive tests**:
```rust
#[cfg(test)]
mod tests {
    #[test]
    fn test_custom_node_functionality() {
        // Test implementation
    }
}
```

### Contributing

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## 📈 Performance

Glint is designed for performance:

- **Zero-copy operations** where possible
- **Minimal allocations** during execution
- **Parallel batch processing** for independent nodes
- **Compile-time optimizations** via Rust

Benchmark results (on modern hardware):
- Simple workflow: ~50μs execution time
- Complex workflow (10+ nodes): ~200μs execution time  
- Memory usage: <10MB for typical workflows
- Test suite: 197 tests run in <25 seconds

## 🔧 Configuration

### Environment Variables
```bash
export GLINT_LOG_LEVEL=info
export GLINT_MAX_PARALLEL_NODES=10
export GLINT_EXECUTION_TIMEOUT=30s
```

### Runtime Configuration
```json
{
    "execution": {
        "max_parallel_nodes": 10,
        "timeout_seconds": 30,
        "retry_attempts": 3
    },
    "logging": {
        "level": "info",
        "format": "json"
    }
}
```

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🤝 Support

- **Documentation**: [docs.glint.dev](https://docs.glint.dev) (coming soon)
- **Issues**: [GitHub Issues](https://github.com/your-username/glint/issues)
- **Discussions**: [GitHub Discussions](https://github.com/your-username/glint/discussions)

## 🗺️ Roadmap

- [ ] Web-based workflow editor
- [ ] More built-in integrations (databases, APIs, cloud services)
- [ ] Workflow templates and marketplace
- [ ] Distributed execution support
- [ ] Real-time workflow monitoring dashboard
- [ ] GraphQL API for workflow management
- [ ] Kubernetes operator for cloud-native deployments
- [ ] Binary data processing nodes
- [ ] Advanced debugging and profiling tools

---

Built with ❤️ in Rust | Empowering developers to automate with confidence