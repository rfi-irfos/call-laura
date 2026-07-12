# laura-mcp

Stdio MCP server exposing a single tool, `review_plan`, backed by
[`call-laura-core`](https://crates.io/crates/call-laura-core) (source: `laura-core/`).

**Read the workspace root README first** for what this is grounded in and,
critically, the "Attribution & Sourcing" section — every lens result tells you
whether it's genuinely Laura Serna Gaviria's own framework or this project's own
addition.

## Install

```bash
cargo install laura-mcp
claude mcp add laura -s user -- laura-mcp
```

No API key or environment setup needed — `call-laura-core` is fully deterministic
and local.

## License

Business Source License 1.1 (non-commercial/research use permitted). See the
workspace root `LICENSE-BSL`.
