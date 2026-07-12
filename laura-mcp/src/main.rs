//! `laura-mcp` — stdio JSON-RPC MCP server exposing a single tool, `review_plan`.
//!
//! All protocol logic (`RpcRequest`/`RpcResponse`, `handle_request`, `tools_list`)
//! lives in `call_laura_core::mcp` and is shared verbatim with `laura-api`'s `/mcp` HTTP
//! endpoint — the two transports can never drift apart in behavior. The only thing
//! this binary owns is the stdio read/write loop.

use call_laura_core::mcp::{handle_request, RpcRequest, RpcResponse};
use serde_json::Value;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

#[tokio::main]
async fn main() {
    eprintln!("[laura-mcp] server ready — review_plan tool, deterministic/local (no API key needed)");
    eprintln!("[laura-mcp] waiting for MCP client on stdin...");

    let stdin = tokio::io::stdin();
    let mut lines = BufReader::new(stdin).lines();
    let mut stdout = tokio::io::stdout();

    loop {
        let line = match lines.next_line().await {
            Ok(Some(l)) if l.trim().is_empty() => continue,
            Ok(Some(l)) => l,
            Ok(None) => break,
            Err(_) => break,
        };

        let response = match serde_json::from_str::<RpcRequest>(&line) {
            Ok(req) => handle_request(req),
            Err(e) => RpcResponse::err(Value::Null, -32700, format!("parse error: {e}")),
        };

        let json_line = serde_json::to_string(&response).unwrap_or_default();
        let _ = stdout.write_all(json_line.as_bytes()).await;
        let _ = stdout.write_all(b"\n").await;
        let _ = stdout.flush().await;
    }
}
