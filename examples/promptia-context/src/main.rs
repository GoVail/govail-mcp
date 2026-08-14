use async_trait::async_trait;
use govail_mcp_contracts::{ContextEnvelope, ToolError};
use govail_mcp_sdk::{GovailMcpServer, ToolHandler};
use serde::{Deserialize, Serialize};
use serde_json::json;

// --- Promptia Domain Payload Structs ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoryContext {
    pub story_id: String,
    pub title: String,
    pub genre: String,
    pub current_arc: String,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Episode {
    pub episode_num: u32,
    pub title: String,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoreItem {
    pub topic: String,
    pub description: String,
}

// --- Handlers ---

struct GetStoryContextHandler;

#[async_trait]
impl ToolHandler for GetStoryContextHandler {
    async fn call(
        &self,
        params: Option<serde_json::Value>,
    ) -> Result<serde_json::Value, ToolError> {
        let story_id = params
            .as_ref()
            .and_then(|p| p.get("story_id"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::invalid_args("Parameter 'story_id' is required"))?;

        if story_id == "unknown" {
            return Err(ToolError::not_found(format!(
                "Story ID '{}' not found",
                story_id
            )));
        }

        let context = StoryContext {
            story_id: story_id.to_string(),
            title: "Cyberpunk Alchemist".to_string(),
            genre: "Sci-Fi Fantasy".to_string(),
            current_arc: "Arc 2: Neon Reclamation".to_string(),
            summary: "An alchemist uncovers ancient tech in a dystopian megalopolis.".to_string(),
        };

        let envelope = ContextEnvelope::new(
            context,
            "promptia",
            format!("story:{}", story_id),
            "get_story_context",
        );

        Ok(serde_json::to_value(envelope).unwrap())
    }
}

struct GetRecentEpisodesHandler;

#[async_trait]
impl ToolHandler for GetRecentEpisodesHandler {
    async fn call(
        &self,
        params: Option<serde_json::Value>,
    ) -> Result<serde_json::Value, ToolError> {
        let story_id = params
            .as_ref()
            .and_then(|p| p.get("story_id"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::invalid_args("Parameter 'story_id' is required"))?;

        let episodes = vec![
            Episode {
                episode_num: 1,
                title: "The Rusting Citadel".to_string(),
                summary: "Hero arrives at Sector 4.".to_string(),
            },
            Episode {
                episode_num: 2,
                title: "Silicon Transmutation".to_string(),
                summary: "Hero synthesizes high-purity core.".to_string(),
            },
        ];

        let envelope = ContextEnvelope::new(
            episodes,
            "promptia",
            format!("story:{}:episodes", story_id),
            "get_recent_episodes",
        );

        Ok(serde_json::to_value(envelope).unwrap())
    }
}

struct SearchLoreHandler;

#[async_trait]
impl ToolHandler for SearchLoreHandler {
    async fn call(
        &self,
        params: Option<serde_json::Value>,
    ) -> Result<serde_json::Value, ToolError> {
        let story_id = params
            .as_ref()
            .and_then(|p| p.get("story_id"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::invalid_args("Parameter 'story_id' is required"))?;

        let query = params
            .as_ref()
            .and_then(|p| p.get("query"))
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let lore = vec![LoreItem {
            topic: "Transmutation Matrix".to_string(),
            description: format!(
                "Lore matching '{}' for story {}: Tech-magic conversion formula.",
                query, story_id
            ),
        }];

        let envelope = ContextEnvelope::new(
            lore,
            "promptia",
            format!("story:{}:lore", story_id),
            "search_lore",
        );

        Ok(serde_json::to_value(envelope).unwrap())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let server = GovailMcpServer::builder("promptia-context-mcp", "0.1.0")
        .tool(
            "get_story_context",
            "Get story metadata, current arc, and summary",
            json!({
                "type": "object",
                "properties": {
                    "story_id": { "type": "string" }
                },
                "required": ["story_id"]
            }),
            GetStoryContextHandler,
        )
        .tool(
            "get_recent_episodes",
            "Get recent episode summaries for a story",
            json!({
                "type": "object",
                "properties": {
                    "story_id": { "type": "string" },
                    "limit": { "type": "number" }
                },
                "required": ["story_id"]
            }),
            GetRecentEpisodesHandler,
        )
        .tool(
            "search_lore",
            "Search lore and world settings for a story",
            json!({
                "type": "object",
                "properties": {
                    "story_id": { "type": "string" },
                    "query": { "type": "string" }
                },
                "required": ["story_id", "query"]
            }),
            SearchLoreHandler,
        )
        .build();

    server.run_stdio().await?;
    Ok(())
}
