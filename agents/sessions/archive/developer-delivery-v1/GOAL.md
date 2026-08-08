# Developer Delivery V1

## Goal

Deliver a standalone CLI workflow that inspects a repository, creates an
allowlisted local commit, prepares an immutable push proposal, records an
explicit approval, revalidates repository and remote state, and pushes safely.

## Done when

- CLI and domain boundaries are documented.
- Commit and push are separate operations.
- Approval is bound to the exact push intent and expires after one use.
- State drift invalidates approval.
- Force and protected-branch pushes fail closed.
- Hermetic local Git tests and a real release build pass.

## Out of scope

- Real GitHub PR/Jira writes
- Slack/GitHub/Jira triggers
- Repository RAG graph ingestion
- GoVail Runtime backend implementation
