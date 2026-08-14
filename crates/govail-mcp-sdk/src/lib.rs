pub use async_trait::async_trait;
pub use govail_mcp_contracts::{
    CapabilityMetadata, CapabilityType, ContextEnvelope, ErrorCode, Freshness, Provenance,
    ResourceIdentity, RiskLevel, ToolError,
};
pub use govail_mcp_core::protocol::{
    JsonRpcError, JsonRpcRequest, JsonRpcResponse, ListToolsResult, ToolDefinition,
};
use std::collections::HashMap;
use std::sync::Arc;

#[async_trait]
pub trait ToolHandler: Send + Sync {
    async fn call(&self, params: Option<serde_json::Value>)
        -> Result<serde_json::Value, ToolError>;
}

struct RegisteredTool {
    definition: ToolDefinition,
    handler: Arc<dyn ToolHandler>,
}

pub struct GovailMcpServer {
    name: String,
    version: String,
    tools: HashMap<String, RegisteredTool>,
}

pub struct GovailMcpServerBuilder {
    name: String,
    version: String,
    tools: HashMap<String, RegisteredTool>,
}

impl GovailMcpServerBuilder {
    pub fn new(name: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            version: version.into(),
            tools: HashMap::new(),
        }
    }

    pub fn tool(
        mut self,
        name: impl Into<String>,
        description: impl Into<String>,
        input_schema: serde_json::Value,
        handler: impl ToolHandler + 'static,
    ) -> Self {
        let name_str = name.into();
        let def = ToolDefinition {
            name: name_str.clone(),
            description: description.into(),
            input_schema,
        };
        self.tools.insert(
            name_str,
            RegisteredTool {
                definition: def,
                handler: Arc::new(handler),
            },
        );
        self
    }

    pub fn build(self) -> GovailMcpServer {
        GovailMcpServer {
            name: self.name,
            version: self.version,
            tools: self.tools,
        }
    }
}

impl GovailMcpServer {
    pub fn builder(name: impl Into<String>, version: impl Into<String>) -> GovailMcpServerBuilder {
        GovailMcpServerBuilder::new(name, version)
    }

    pub async fn handle_request(&self, req: JsonRpcRequest) -> Option<JsonRpcResponse> {
        let is_notification = req.id.is_none();

        match req.method.as_str() {
            "initialize" => {
                let requested_version = req
                    .params
                    .as_ref()
                    .and_then(|p| p.get("protocolVersion"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("2025-11-25");

                let init_res = serde_json::json!({
                    "protocolVersion": requested_version,
                    "capabilities": {
                        "tools": {}
                    },
                    "serverInfo": {
                        "name": self.name,
                        "version": self.version
                    }
                });
                Some(JsonRpcResponse::success(req.id, init_res))
            }
            "notifications/initialized" => {
                // MCP specification: notification does not expect a response
                None
            }
            "tools/list" => {
                let definitions: Vec<ToolDefinition> =
                    self.tools.values().map(|t| t.definition.clone()).collect();
                let list_res =
                    serde_json::to_value(ListToolsResult { tools: definitions }).unwrap();
                Some(JsonRpcResponse::success(req.id, list_res))
            }
            "tools/call" => {
                let params = req.params.clone().unwrap_or(serde_json::Value::Null);
                let tool_name = params.get("name").and_then(|n| n.as_str());
                let tool_args = params.get("arguments").cloned();

                let name = match tool_name {
                    Some(n) => n,
                    None => {
                        return Some(JsonRpcResponse::error(
                            req.id,
                            JsonRpcError::invalid_params("Missing tool 'name' in parameters"),
                        ));
                    }
                };

                match self.tools.get(name) {
                    Some(tool) => match tool.handler.call(tool_args).await {
                        Ok(res) => Some(JsonRpcResponse::success(
                            req.id,
                            serde_json::json!({
                                "content": [
                                    {
                                        "type": "text",
                                        "text": serde_json::to_string_pretty(&res).unwrap_or_default()
                                    }
                                ]
                            }),
                        )),
                        Err(err) => Some(JsonRpcResponse::success(
                            req.id,
                            serde_json::json!({
                                "isError": true,
                                "content": [
                                    {
                                        "type": "text",
                                        "text": serde_json::to_string_pretty(&err).unwrap_or_default()
                                    }
                                ]
                            }),
                        )),
                    },
                    None => Some(JsonRpcResponse::error(
                        req.id,
                        JsonRpcError::method_not_found(format!("Tool '{}' not found", name)),
                    )),
                }
            }
            _ => {
                if is_notification {
                    None
                } else {
                    Some(JsonRpcResponse::error(
                        req.id,
                        JsonRpcError::method_not_found(format!(
                            "Method '{}' not supported",
                            req.method
                        )),
                    ))
                }
            }
        }
    }

    pub async fn run_stdio(self) -> Result<(), Box<dyn std::error::Error>> {
        let (stdin, stdout) = rmcp::transport::io::stdio();
        let transport = rmcp::transport::async_rw::AsyncRwTransport::new(stdin, stdout);
        let service = rmcp::service::serve_server(self, transport).await?;
        let _ = service.waiting().await;
        Ok(())
    }
}

impl rmcp::handler::server::ServerHandler for GovailMcpServer {
    async fn initialize(
        &self,
        _request: rmcp::model::InitializeRequestParams,
        _context: rmcp::service::RequestContext<rmcp::service::RoleServer>,
    ) -> Result<rmcp::model::InitializeResult, rmcp::model::ErrorData> {
        let mut capabilities = rmcp::model::ServerCapabilities::default();
        capabilities.tools = Some(rmcp::model::ToolsCapability::default());
        let server_info = rmcp::model::Implementation::new(self.name.clone(), self.version.clone());
        Ok(rmcp::model::InitializeResult::new(capabilities)
            .with_server_info(server_info)
            .with_protocol_version(rmcp::model::ProtocolVersion::V_2025_11_25))
    }

    async fn list_tools(
        &self,
        _request: Option<rmcp::model::PaginatedRequestParams>,
        _context: rmcp::service::RequestContext<rmcp::service::RoleServer>,
    ) -> Result<rmcp::model::ListToolsResult, rmcp::model::ErrorData> {
        let tools: Vec<rmcp::model::Tool> = self
            .tools
            .values()
            .map(|t| {
                rmcp::model::Tool::new(
                    t.definition.name.clone(),
                    t.definition.description.clone(),
                    std::sync::Arc::new(
                        t.definition
                            .input_schema
                            .as_object()
                            .cloned()
                            .unwrap_or_default(),
                    ),
                )
            })
            .collect();
        Ok(rmcp::model::ListToolsResult::with_all_items(tools))
    }

    async fn call_tool(
        &self,
        request: rmcp::model::CallToolRequestParams,
        _context: rmcp::service::RequestContext<rmcp::service::RoleServer>,
    ) -> Result<rmcp::model::CallToolResponse, rmcp::model::ErrorData> {
        let tool_name = request.name.as_ref();
        match self.tools.get(tool_name) {
            Some(tool) => {
                let args_val = request.arguments.map(serde_json::Value::Object);
                match tool.handler.call(args_val).await {
                    Ok(res) => {
                        let text = serde_json::to_string_pretty(&res).unwrap_or_default();
                        let result = rmcp::model::CallToolResult::success(vec![
                            rmcp::model::ContentBlock::Text(rmcp::model::TextContent::new(text)),
                        ]);
                        Ok(rmcp::model::CallToolResponse::from(result))
                    }
                    Err(err) => {
                        let text = serde_json::to_string_pretty(&err).unwrap_or_default();
                        let result = rmcp::model::CallToolResult::error(vec![
                            rmcp::model::ContentBlock::Text(rmcp::model::TextContent::new(text)),
                        ]);
                        Ok(rmcp::model::CallToolResponse::from(result))
                    }
                }
            }
            None => Err(rmcp::model::ErrorData::new(
                rmcp::model::ErrorCode::METHOD_NOT_FOUND,
                format!("Tool '{}' not found", tool_name),
                None,
            )),
        }
    }
}
