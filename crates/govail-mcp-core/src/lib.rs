use serde::{Deserialize, Serialize};

pub mod protocol {
    use super::*;

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct JsonRpcRequest {
        pub jsonrpc: String,
        pub id: Option<serde_json::Value>,
        pub method: String,
        #[serde(default)]
        pub params: Option<serde_json::Value>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct JsonRpcResponse {
        pub jsonrpc: String,
        pub id: Option<serde_json::Value>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub result: Option<serde_json::Value>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub error: Option<JsonRpcError>,
    }

    impl JsonRpcResponse {
        pub fn success(id: Option<serde_json::Value>, result: serde_json::Value) -> Self {
            Self {
                jsonrpc: "2.0".to_string(),
                id,
                result: Some(result),
                error: None,
            }
        }

        pub fn error(id: Option<serde_json::Value>, error: JsonRpcError) -> Self {
            Self {
                jsonrpc: "2.0".to_string(),
                id,
                result: None,
                error: Some(error),
            }
        }
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct JsonRpcError {
        pub code: i32,
        pub message: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub data: Option<serde_json::Value>,
    }

    impl JsonRpcError {
        pub fn invalid_request(msg: impl Into<String>) -> Self {
            Self {
                code: -32600,
                message: msg.into(),
                data: None,
            }
        }

        pub fn method_not_found(msg: impl Into<String>) -> Self {
            Self {
                code: -32601,
                message: msg.into(),
                data: None,
            }
        }

        pub fn invalid_params(msg: impl Into<String>) -> Self {
            Self {
                code: -32602,
                message: msg.into(),
                data: None,
            }
        }

        pub fn internal_error(msg: impl Into<String>) -> Self {
            Self {
                code: -32603,
                message: msg.into(),
                data: None,
            }
        }
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct ToolDefinition {
        pub name: String,
        pub description: String,
        #[serde(rename = "inputSchema")]
        pub input_schema: serde_json::Value,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct ListToolsResult {
        pub tools: Vec<ToolDefinition>,
    }
}
