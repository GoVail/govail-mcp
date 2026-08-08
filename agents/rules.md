# GoVail MCP Project Rules

## Boot order

1. Read this file.
2. Read `docs/architecture.md`.
3. For active work, read `agents/sessions/active/<work-id>/`.

## Product boundaries

- `govail-mcp` is an externally consumable MCP and CLI product.
- `developer` owns developer-facing workflow contracts and orchestration ports.
- Standalone execution and GoVail Runtime delegation must implement the same
  workflow contract.
- MCP and CLI handlers must call the same application services.
- Git, SCM, issue tracker, LLM, graph store, and trigger integrations are
  replaceable adapters.

## Security

- Models propose actions; deterministic policy and executors decide and act.
- Never persist raw source or full diffs in central service state.
- Never stage with `git add -A` or an implicit workspace-wide glob.
- Commit and push are separate capabilities. Push requires an approval receipt
  bound to repository, remote, commit, branch, remote head, and verification.
- Revalidate every bound value immediately before push and fail closed on drift.
- V1 forbids force pushes, deletion pushes, and direct pushes to protected
  branches.
- Never commit real credentials, private addresses, or `.env` files.

## Development

- Design first: update an architecture document before functional code.
- Keep tests hermetic. Git write tests must use temporary local repositories and
  local bare remotes; never contact or mutate a real remote.
- Run `cargo fmt --check`, `cargo test --locked`, and `cargo build --locked`
  before claiming code completion.
- Production deployment and live HTTP verification occur on the designated
  service host through Compose. Report unperformed verification explicitly.
