# GoVail MCP — AI Collaboration Guide

Before design, analysis, or code changes, read `agents/rules.md`.

The repository exposes GoVail and standalone developer automation through MCP
and CLI entrypoints. Product logic belongs in domain/application modules; MCP,
CLI, database, Git, LLM, and GoVail Runtime integrations are adapters.
