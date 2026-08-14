# GoVail MCP Contracts Specification

## 1. Overview

`govail-mcp-contracts` defines standard, serializable data structures exchanged between Applications, MCP Servers, and GoVail Core.

## 2. Protocol Version Baseline

- **Baseline Specification**: `MCP 2025-11-25` (Legacy `initialize` session protocol)
- **Protocol Boundary**: Wire protocol parsing is delegated to official SDKs (`rmcp`), while `govail-mcp` owns interop conventions.

## 3. Core Envelope Structures

### `ContextEnvelope<T>`

Wraps retrieved context data with provenance, freshness, and resource identity metadata.

```json
{
  "data": { ... },
  "provenance": {
    "source": "promptia",
    "resource": "story:101",
    "retrieved_at": "2026-08-14T10:00:00Z"
  },
  "freshness": {
    "observed_at": "2026-08-14T10:00:00Z",
    "stale": false
  },
  "metadata": {
    "name": "get_story_context",
    "capability_type": "READ",
    "risk_level": "LOW",
    "requires_approval": false
  }
}
```

### `ToolError`

Standardized error response for all MCP tool invocations.

```json
{
  "code": "RESOURCE_NOT_FOUND",
  "message": "Story ID 999 does not exist",
  "details": null,
  "retryable": false
}
```

### `CapabilityMetadata`

Describes tool properties including read/write classification, risk rating, and required governance attributes.

```json
{
  "name": "get_story_context",
  "capability_type": "READ",
  "risk_level": "LOW",
  "requires_approval": false
}
```
