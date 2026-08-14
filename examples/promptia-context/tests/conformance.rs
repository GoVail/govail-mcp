use async_trait::async_trait;
use govail_mcp_contracts::{CapabilityType, ContextEnvelope, RiskLevel, ToolError};
use govail_mcp_core::protocol::JsonRpcRequest;
use govail_mcp_sdk::{GovailMcpServer, ToolHandler};
use serde_json::json;

struct TestStoryHandler;

#[async_trait]
impl ToolHandler for TestStoryHandler {
    async fn call(
        &self,
        params: Option<serde_json::Value>,
    ) -> Result<serde_json::Value, ToolError> {
        let story_id = params
            .as_ref()
            .and_then(|p| p.get("story_id"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::invalid_args("Missing story_id"))?;

        if story_id == "invalid" {
            return Err(ToolError::not_found("Story not found"));
        }

        let envelope = ContextEnvelope::new(
            json!({ "title": "Cyberpunk Alchemist" }),
            "promptia",
            format!("story:{}", story_id),
            "get_story_context",
        );
        Ok(serde_json::to_value(envelope).unwrap())
    }
}

// --- Gate 3: Internal MCP E2E Test ---
#[tokio::test]
async fn test_g3_internal_mcp_e2e() {
    let server = GovailMcpServer::builder("promptia-test-server", "1.0.0")
        .tool(
            "get_story_context",
            "Get story context",
            json!({"type": "object"}),
            TestStoryHandler,
        )
        .build();

    // 1. Initialize (MCP 2025-11-25 Legacy Baseline)
    let init_req = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: Some(json!(1)),
        method: "initialize".to_string(),
        params: None,
    };
    let init_res = server.handle_request(init_req).await.unwrap();
    assert!(init_res.result.is_some());
    let init_val = init_res.result.unwrap();
    assert_eq!(init_val.get("protocolVersion").unwrap(), "2025-11-25");

    // 2. Tools List
    let list_req = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: Some(json!(2)),
        method: "tools/list".to_string(),
        params: None,
    };
    let list_res = server.handle_request(list_req).await.unwrap();
    assert!(list_res.result.is_some());

    // 3. Tools Call Success
    let call_req = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: Some(json!(3)),
        method: "tools/call".to_string(),
        params: Some(json!({
            "name": "get_story_context",
            "arguments": { "story_id": "101" }
        })),
    };
    let call_res = server.handle_request(call_req).await.unwrap();
    assert!(call_res.result.is_some());

    // 4. Tools Call Error Normalization
    let err_req = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: Some(json!(4)),
        method: "tools/call".to_string(),
        params: Some(json!({
            "name": "get_story_context",
            "arguments": { "story_id": "invalid" }
        })),
    };
    let err_res = server.handle_request(err_req).await.unwrap();
    assert!(err_res.result.is_some());
    let err_str = err_res.result.unwrap().to_string();
    assert!(err_str.contains("isError"));
    assert!(err_str.contains("RESOURCE_NOT_FOUND"));
}

// --- Gate 4: MCP Wire-format Interop Test (Standard JSON-RPC 2.0 wire packet) ---
#[tokio::test]
async fn test_g4_mcp_wire_format_interop() {
    let server = GovailMcpServer::builder("promptia-context-mcp", "0.1.0")
        .tool(
            "get_story_context",
            "Get story metadata",
            json!({"type": "object"}),
            TestStoryHandler,
        )
        .build();

    // Simulate external MCP client sending standard JSON-RPC wire payload
    let wire_payload = json!({
        "jsonrpc": "2.0",
        "id": 100,
        "method": "tools/call",
        "params": {
            "name": "get_story_context",
            "arguments": { "story_id": "777" }
        }
    });

    let req: JsonRpcRequest = serde_json::from_value(wire_payload).unwrap();
    let resp = server.handle_request(req).await.unwrap();

    assert_eq!(resp.jsonrpc, "2.0");
    assert_eq!(resp.id, Some(json!(100)));
    assert!(resp.error.is_none());
    assert!(resp.result.is_some());

    let content_text = resp.result.unwrap()["content"][0]["text"]
        .as_str()
        .unwrap()
        .to_string();
    assert!(content_text.contains("promptia"));
    assert!(content_text.contains("story:777"));
}

// --- Gate 5: Strict Contract & Envelope Validation Test ---
#[tokio::test]
async fn test_g5_contract_and_envelope_validation() {
    let server = GovailMcpServer::builder("promptia-context-mcp", "0.1.0")
        .tool(
            "get_story_context",
            "Get story metadata",
            json!({"type": "object"}),
            TestStoryHandler,
        )
        .build();

    let call_req = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: Some(json!(1)),
        method: "tools/call".to_string(),
        params: Some(json!({
            "name": "get_story_context",
            "arguments": { "story_id": "story_spec_1" }
        })),
    };

    let resp = server.handle_request(call_req).await.unwrap();
    let text = resp.result.unwrap()["content"][0]["text"]
        .as_str()
        .unwrap()
        .to_string();

    // Deserialize into ContextEnvelope<serde_json::Value> to validate strictly
    let envelope: ContextEnvelope<serde_json::Value> = serde_json::from_str(&text).unwrap();

    assert_eq!(envelope.provenance.source, "promptia");
    assert_eq!(envelope.provenance.resource, "story:story_spec_1");
    assert!(!envelope.freshness.stale);
    assert_eq!(envelope.metadata.name, "get_story_context");
    assert_eq!(envelope.metadata.capability_type, CapabilityType::Read);
    assert_eq!(envelope.metadata.risk_level, RiskLevel::Low);
    assert!(!envelope.metadata.requires_approval);
    assert_eq!(envelope.data["title"], "Cyberpunk Alchemist");
}

// --- Gate 6: Official rmcp Client Interop Test ---
struct TestClientHandler;

impl rmcp::handler::client::ClientHandler for TestClientHandler {}

#[tokio::test]
async fn test_g6_official_rmcp_client_interop() {
    let server = GovailMcpServer::builder("promptia-context-mcp", "0.1.0")
        .tool(
            "get_story_context",
            "Get story metadata, current arc, and summary",
            json!({
                "type": "object",
                "properties": { "story_id": { "type": "string" } },
                "required": ["story_id"]
            }),
            TestStoryHandler,
        )
        .build();

    let (client_read, server_write) = tokio::io::duplex(8192);
    let (server_read, client_write) = tokio::io::duplex(8192);

    let server_transport =
        rmcp::transport::async_rw::AsyncRwTransport::new(server_read, server_write);
    let server_task = tokio::spawn(async move {
        if let Ok(service) = rmcp::service::serve_server(server, server_transport).await {
            let _ = service.waiting().await;
        }
    });

    let client_transport =
        rmcp::transport::async_rw::AsyncRwTransport::new(client_read, client_write);
    let running_client = rmcp::service::serve_client(TestClientHandler, client_transport)
        .await
        .expect("Failed to start rmcp client");

    // 1. Initialize is handled by rmcp during handshake / initialize
    let init_info = running_client.peer().peer_info();
    assert!(init_info.is_some());
    let info = init_info.unwrap();
    assert_eq!(
        info.server_info.as_ref().unwrap().name,
        "promptia-context-mcp"
    );
    assert_eq!(info.server_info.as_ref().unwrap().version, "0.1.0");

    // 2. List tools via rmcp client
    let tools_res = running_client
        .peer()
        .list_tools(None)
        .await
        .expect("rmcp list_tools failed");
    assert_eq!(tools_res.tools.len(), 1);
    assert_eq!(tools_res.tools[0].name, "get_story_context");

    // 3. Call tool via rmcp client
    let call_res = running_client
        .peer()
        .call_tool(
            rmcp::model::CallToolRequestParams::new("get_story_context").with_arguments(
                json!({ "story_id": "interop_42" })
                    .as_object()
                    .unwrap()
                    .clone(),
            ),
        )
        .await
        .expect("rmcp call_tool failed");

    assert!(!call_res.is_error.unwrap_or(false));
    assert!(!call_res.content.is_empty());
    if let rmcp::model::ContentBlock::Text(text_content) = &call_res.content[0] {
        let envelope: ContextEnvelope<serde_json::Value> =
            serde_json::from_str(&text_content.text).expect("ContextEnvelope decoding failed");
        assert_eq!(envelope.provenance.source, "promptia");
        assert_eq!(envelope.provenance.resource, "story:interop_42");
        assert_eq!(envelope.data["title"], "Cyberpunk Alchemist");
    } else {
        panic!("Expected text content block");
    }

    // 4. Call tool with error via rmcp client
    let err_call_res = running_client
        .peer()
        .call_tool(
            rmcp::model::CallToolRequestParams::new("get_story_context").with_arguments(
                json!({ "story_id": "invalid" })
                    .as_object()
                    .unwrap()
                    .clone(),
            ),
        )
        .await
        .expect("rmcp call_tool error path failed");

    assert_eq!(err_call_res.is_error, Some(true));
    if let rmcp::model::ContentBlock::Text(text_content) = &err_call_res.content[0] {
        let err_obj: ToolError =
            serde_json::from_str(&text_content.text).expect("ToolError decoding failed");
        assert_eq!(
            err_obj.code,
            govail_mcp_contracts::ErrorCode::ResourceNotFound
        );
    } else {
        panic!("Expected text content block for error");
    }

    // Cleanup
    drop(running_client);
    server_task.abort();
}

// --- Gate 1: Baseline Contracts Freeze & Snapshot Test ---
#[test]
fn test_g1_baseline_contracts_freeze() {
    let envelope = ContextEnvelope::new(
        json!({ "title": "Cyberpunk Alchemist", "genre": "Sci-Fi" }),
        "promptia",
        "story:spec_42",
        "get_story_context",
    );
    let serialized = serde_json::to_value(&envelope).unwrap();
    assert_eq!(serialized["data"]["title"], "Cyberpunk Alchemist");
    assert_eq!(serialized["provenance"]["source"], "promptia");
    assert_eq!(serialized["provenance"]["resource"], "story:spec_42");
    assert_eq!(serialized["freshness"]["stale"], false);
    assert_eq!(serialized["metadata"]["name"], "get_story_context");
    assert_eq!(serialized["metadata"]["capability_type"], "READ");
    assert_eq!(serialized["metadata"]["risk_level"], "LOW");
    assert_eq!(serialized["metadata"]["requires_approval"], false);

    let not_found_err = ToolError::not_found("Story 42 missing");
    let err_val = serde_json::to_value(&not_found_err).unwrap();
    assert_eq!(err_val["code"], "RESOURCE_NOT_FOUND");
    assert_eq!(err_val["message"], "Story 42 missing");
    assert_eq!(err_val["retryable"], false);

    let invalid_args_err = ToolError::invalid_args("Parameter missing");
    let err_val2 = serde_json::to_value(&invalid_args_err).unwrap();
    assert_eq!(err_val2["code"], "INVALID_ARGUMENTS");
    assert_eq!(err_val2["message"], "Parameter missing");
    assert_eq!(err_val2["retryable"], false);
}

// --- Gate 7: Promptia Reference Regression Test ---
#[tokio::test]
async fn test_g7_promptia_reference_regression() {
    struct TestEpisodesHandler;
    #[async_trait]
    impl ToolHandler for TestEpisodesHandler {
        async fn call(
            &self,
            params: Option<serde_json::Value>,
        ) -> Result<serde_json::Value, ToolError> {
            let story_id = params
                .as_ref()
                .and_then(|p| p.get("story_id"))
                .and_then(|v| v.as_str())
                .ok_or_else(|| ToolError::invalid_args("Missing story_id"))?;

            let envelope = ContextEnvelope::new(
                vec![json!({ "episode_num": 1, "title": "The Rusting Citadel" })],
                "promptia",
                format!("story:{}:episodes", story_id),
                "get_recent_episodes",
            );
            Ok(serde_json::to_value(envelope).unwrap())
        }
    }

    struct TestLoreHandler;
    #[async_trait]
    impl ToolHandler for TestLoreHandler {
        async fn call(
            &self,
            params: Option<serde_json::Value>,
        ) -> Result<serde_json::Value, ToolError> {
            let story_id = params
                .as_ref()
                .and_then(|p| p.get("story_id"))
                .and_then(|v| v.as_str())
                .ok_or_else(|| ToolError::invalid_args("Missing story_id"))?;

            let envelope = ContextEnvelope::new(
                vec![json!({ "topic": "Transmutation Matrix" })],
                "promptia",
                format!("story:{}:lore", story_id),
                "search_lore",
            );
            Ok(serde_json::to_value(envelope).unwrap())
        }
    }

    let server = GovailMcpServer::builder("promptia-context-mcp", "0.1.0")
        .tool(
            "get_story_context",
            "Story context",
            json!({"type": "object"}),
            TestStoryHandler,
        )
        .tool(
            "get_recent_episodes",
            "Episodes",
            json!({"type": "object"}),
            TestEpisodesHandler,
        )
        .tool(
            "search_lore",
            "Lore",
            json!({"type": "object"}),
            TestLoreHandler,
        )
        .build();

    // Verify all 3 tools are listed
    let list_req = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: Some(json!(10)),
        method: "tools/list".to_string(),
        params: None,
    };
    let list_res = server.handle_request(list_req).await.unwrap();
    let tools = list_res.result.unwrap()["tools"]
        .as_array()
        .unwrap()
        .clone();
    assert_eq!(tools.len(), 3);

    // Verify calling episodes tool
    let ep_req = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: Some(json!(11)),
        method: "tools/call".to_string(),
        params: Some(json!({
            "name": "get_recent_episodes",
            "arguments": { "story_id": "99" }
        })),
    };
    let ep_res = server.handle_request(ep_req).await.unwrap();
    let ep_text = ep_res.result.unwrap()["content"][0]["text"]
        .as_str()
        .unwrap()
        .to_string();
    let ep_envelope: ContextEnvelope<serde_json::Value> = serde_json::from_str(&ep_text).unwrap();
    assert_eq!(ep_envelope.provenance.resource, "story:99:episodes");

    // Verify calling lore tool
    let lore_req = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: Some(json!(12)),
        method: "tools/call".to_string(),
        params: Some(json!({
            "name": "search_lore",
            "arguments": { "story_id": "99" }
        })),
    };
    let lore_res = server.handle_request(lore_req).await.unwrap();
    let lore_text = lore_res.result.unwrap()["content"][0]["text"]
        .as_str()
        .unwrap()
        .to_string();
    let lore_envelope: ContextEnvelope<serde_json::Value> =
        serde_json::from_str(&lore_text).unwrap();
    assert_eq!(lore_envelope.provenance.resource, "story:99:lore");
}
