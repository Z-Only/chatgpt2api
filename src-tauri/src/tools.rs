use serde_json::{json, Value};

pub fn convert_openai_tools(tools: &[Value]) -> Vec<Value> {
    tools
        .iter()
        .map(|tool| {
            if tool.get("type").and_then(Value::as_str) != Some("function") {
                return tool.clone();
            }

            let function = &tool["function"];
            json!({
                "type": "function",
                "name": function["name"],
                "description": function["description"],
                "parameters": function["parameters"],
            })
        })
        .collect()
}
