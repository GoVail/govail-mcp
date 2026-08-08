# GoVail MCP

GoVail MCP is an extensible developer automation product exposed through a CLI
and, in later slices, Model Context Protocol tools.

The first implementation is Developer Delivery V1: an allowlisted local commit
followed by an immutable, explicitly approved push.

## Current commands

```bash
cargo run --bin govail -- developer inspect --repo /path/to/repository

cargo run --bin govail -- developer commit \
  --repo /path/to/repository \
  --message "feat: add workflow" \
  --file src/workflow.rs \
  --file tests/workflow.rs

cargo run --bin govail -- developer verify \
  --repo /path/to/repository \
  --name tests \
  -- cargo test --locked

cargo run --bin govail -- developer propose-push \
  --repo /path/to/repository \
  --remote origin \
  --branch feature/delivery \
  --verification <attestation-id>

cargo run --bin govail -- developer approve \
  --repo /path/to/repository \
  --proposal <proposal-id> \
  --yes

cargo run --bin govail -- developer push \
  --repo /path/to/repository \
  --approval <approval-id>
```

Workflow state and signing material are stored beneath the target repository's
Git metadata and are not added to its working tree.

## Safety defaults

- explicit file allowlists only; pre-staged changes are rejected
- verification is bound to the exact commit
- push approval is signed, expiring, and single-use
- local HEAD, remote URL, and remote HEAD are checked again before push
- protected branches and non-fast-forward updates are rejected
- tests use temporary repositories and local bare remotes only

See [Developer Delivery V1](docs/developer-delivery-v1.md) and
[Architecture](docs/architecture.md).
