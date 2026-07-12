//! `laura-mcp` — stdio JSON-RPC MCP server.
//!
//! Exposes two tools:
//!   * `review_plan`  — the free, LGPL deterministic 4-lens core (via `call_laura_core`).
//!   * `review_team`  — the proprietary 15-agent "SWAT team" + Laura orchestrator
//!                     (the paid module, BSL-1.1, in `laura-team`).
//!
//! All shared protocol logic (`RpcRequest`/`RpcResponse`, `handle_request` for the
//! free tool, the four-lens `tools_list`) lives in `call_laura_core::mcp` and is
//! reused verbatim — the two transports can never drift apart in behavior. This
//! binary owns only the stdio loop and the team tool's registration/dispatch, which
//! is kept out of `call_laura_core` on purpose so the LGPL core never depends on the
//! proprietary `laura-team` crate.

use call_laura_core::mcp::{handle_request, RpcRequest, RpcResponse};
use laura_team::orchestrator::{review_team, TeamRequest};
use serde_json::Value;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

/// `tools/call` handler for the proprietary team tool.
fn tool_review_team(params: &Value) -> Result<Value, String> {
    let req: TeamRequest = serde_json::from_value(params.clone())
        .map_err(|e| format!("invalid arguments for review_team: {e}"))?;
    let response = review_team(&req);
    serde_json::to_value(&response).map_err(|e| format!("failed to serialize response: {e}"))
}

/// Merge the team tool into the core's `tools/list` payload.
fn tools_list_with_team(core_list: Value) -> Value {
    let mut tools = match core_list.get("tools").and_then(|t| t.as_array()) {
        Some(arr) => arr.clone(),
        None => Vec::new(),
    };
    tools.push(serde_json::json!({
        "name": "review_team",
        "description": "Call Laura's proprietary 15-agent expert team (OSINT, security, legal/compliance, finance, ops, strategy, UX, data-privacy, ethics, threat-model, reliability, hiring, brand, research-method, product). Fully deterministic and local: same input always produces the same output, no network call, no API key at analysis time. Returns per-agent findings (each citing the exact quoted span it reacts to), a risk band, cross-cutting themes where multiple agents converge on the same text region, and a deduplicated priority-action list. This is the paid analysis module (BSL-1.1); the free `review_plan` tool is the 4-lens core.",
        "annotations": {
            "title": "Run Laura's 15-agent team review",
            "readOnlyHint": true,
            "idempotentHint": true,
            "destructiveHint": false,
            "openWorldHint": false
        },
        "inputSchema": {
            "type": "object",
            "properties": {
                "text": {
                    "type": "string",
                    "description": "The plan/document to review, plain text or markdown."
                },
                "agents": {
                    "type": "array",
                    "items": { "type": "string", "enum": [
                        "osint", "security", "legal_compliance", "finance", "ops", "strategy",
                        "ux", "data_privacy", "ethics", "threat_model", "reliability", "hiring",
                        "brand", "research_method", "product"
                    ] },
                    "description": "Which agents to run. Omit to run all 15 (the full team)."
                },
                "metadata": {
                    "type": "object",
                    "properties": {
                        "title": { "type": "string" },
                        "context": { "type": "string" },
                        "caller_tier": { "type": "string", "description": "Reserved for a future paid caller tier; currently informational only." }
                    }
                }
            },
            "required": ["text"]
        }
    }));
    serde_json::json!({ "tools": tools })
}

/// Intercept the two list/call methods so the team tool lives alongside the core
/// tools; everything else (initialize, notifications, error cases) delegates to core.
fn handle_request_with_team(req: RpcRequest) -> RpcResponse {
    match req.method.as_str() {
        "tools/list" => {
            let core_list = call_laura_core::mcp::tools_list();
            RpcResponse::ok(req.id.unwrap_or(Value::Null), tools_list_with_team(core_list))
        }
        "tools/call" => {
            let id = req.id.clone().unwrap_or(Value::Null);
            let params = req.params.clone().unwrap_or(Value::Object(Default::default()));
            let tool_name = match params.get("name").and_then(|n| n.as_str()) {
                Some(n) => n.to_string(),
                None => return RpcResponse::err(id, -32602, "missing tool name"),
            };
            if tool_name == "review_team" {
                let tool_params = params.get("arguments").unwrap_or(&Value::Null);
                return match tool_review_team(tool_params) {
                    Ok(result) => RpcResponse::ok(
                        id,
                        serde_json::json!({ "content": [{ "type": "text", "text": serde_json::to_string_pretty(&result).unwrap_or_default() }] }),
                    ),
                    Err(e) => RpcResponse::err(id, -32000, e),
                };
            }
            // Free-tool path: delegate the whole request to core (review_plan etc.).
            // Reconstruct so we don't fight borrow-checker over a partial move.
            handle_request(RpcRequest {
                jsonrpc: req.jsonrpc.clone(),
                id: req.id.clone(),
                method: req.method.clone(),
                params: Some(params),
            })
        }
        _ => handle_request(req),
    }
}

#[tokio::main]
async fn main() {
    eprintln!("[laura-mcp] server ready — review_plan (free, 4-lens) + review_team (paid, 15-agent) tools, deterministic/local");
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
            Ok(req) => handle_request_with_team(req),
            Err(e) => RpcResponse::err(Value::Null, -32700, format!("parse error: {e}")),
        };

        let json_line = serde_json::to_string(&response).unwrap_or_default();
        let _ = stdout.write_all(json_line.as_bytes()).await;
        let _ = stdout.write_all(b"\n").await;
        let _ = stdout.flush().await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use call_laura_core::mcp::RpcRequest;
    use serde_json::json;

    fn req(method: &str, params: Option<Value>) -> RpcRequest {
        RpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(1)),
            method: method.to_string(),
            params,
        }
    }

    #[test]
    fn tools_list_includes_review_team() {
        let resp = handle_request_with_team(req("tools/list", None));
        let tools = resp.result.as_ref().unwrap()["tools"].as_array().unwrap();
        let names: Vec<&str> = tools.iter().filter_map(|t| t["name"].as_str()).collect();
        assert!(names.contains(&"review_plan"), "free tool must remain listed");
        assert!(names.contains(&"review_team"), "paid team tool must be listed");
    }

    #[test]
    fn review_team_dispatches_via_mcp() {
        let resp = handle_request_with_team(req(
            "tools/call",
            Some(json!({ "name": "review_team", "arguments": { "text": "We store personal data without consent. password=hunter2" } })),
        ));
        assert!(resp.error.is_none(), "review_team should succeed");
        let text = resp.result.as_ref().unwrap()["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("risk_band") || text.contains("\"synthesis\""), "team synthesis should be present");
    }

    #[test]
    fn free_review_plan_still_routes_to_core() {
        let resp = handle_request_with_team(req(
            "tools/call",
            Some(json!({ "name": "review_plan", "arguments": { "text": "# Goals\nShip.\n\n# Risks\nBreak." } })),
        ));
        assert!(resp.error.is_none(), "review_plan must still work");
    }
}
