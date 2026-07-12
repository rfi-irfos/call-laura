//! Shared MCP JSON-RPC protocol logic — used identically by `laura-mcp` (stdio
//! transport) and `laura-api` (HTTP transport, `POST /mcp`, matching what Smithery
//! expects: `type: http` pointing at a URL that itself speaks MCP JSON-RPC, not a
//! bespoke REST shape). Keeping this in `laura-core` means both transports are
//! guaranteed to expose the identical tool behavior — no risk of the stdio and
//! HTTP surfaces drifting apart.

use crate::schema::ReviewRequest;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Deserialize)]
#[allow(dead_code)]
pub struct RpcRequest {
    pub jsonrpc: String,
    pub id: Option<Value>,
    pub method: String,
    pub params: Option<Value>,
}

#[derive(Serialize)]
pub struct RpcResponse {
    pub jsonrpc: String,
    pub id: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<RpcError>,
}

#[derive(Serialize)]
pub struct RpcError {
    pub code: i32,
    pub message: String,
}

impl RpcResponse {
    pub fn ok(id: Value, result: Value) -> Self {
        Self { jsonrpc: "2.0".into(), id, result: Some(result), error: None }
    }
    pub fn err(id: Value, code: i32, message: impl Into<String>) -> Self {
        Self { jsonrpc: "2.0".into(), id, result: None, error: Some(RpcError { code, message: message.into() }) }
    }
}

fn tool_review_plan(params: &Value) -> Result<Value, String> {
    let req: ReviewRequest =
        serde_json::from_value(params.clone()).map_err(|e| format!("invalid arguments for review_plan: {e}"))?;
    if req.text.trim().is_empty() {
        return Err("text must not be empty".to_string());
    }
    if req.lenses.is_empty() {
        return Err("lenses must not be an empty array — omit the field entirely to run all four".to_string());
    }
    let response = crate::review(&req);
    serde_json::to_value(&response).map_err(|e| format!("failed to serialize response: {e}"))
}

fn dispatch_tool(name: &str, params: &Value) -> Result<Value, String> {
    match name {
        "review_plan" => tool_review_plan(params),
        _ => Err(format!("unknown tool: {name}")),
    }
}

pub fn tools_list() -> Value {
    json!({ "tools": [
        {
            "name": "review_plan",
            "description": "Structured review of a plan or document, grounded in Laura Serna Gaviria's published Human-AI Co-Evolution research framework (the OSF preprint's User Integrity Protocol and 8-Layer Model), plus two RFI-IRFOS-original lenses (resonance, ecocentric). Fully deterministic and local: same input always produces the same output, no network call, no API key required. Returns per-lens findings, each citing the exact quoted span of your text it reacts to, plus an overall summary. No opaque single score. Every lens result self-discloses its `source` — 'laura-8layer-2025'/'laura-uip-2025' (directly hers), 'rfi-irfos-operationalization' (this project's own operationalization of a concept she names), or 'rfi-irfos-addition' (not from her framework at all — currently only 'ecocentric'). Read the attribution_note on each lens before treating any finding as authoritative research output — note that the deterministic, keyword-based approach trades semantic nuance for full reproducibility and transparency.",
            "annotations": {
                "title": "Review a plan against Laura's framework",
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
                        "description": "The plan/document to review, plain text or markdown. Markdown headings are used to split into sections for the eight_layer and resonance lenses; without headings, blank-line-separated paragraphs are used instead."
                    },
                    "lenses": {
                        "type": "array",
                        "items": { "type": "string", "enum": ["eight_layer", "uip_check", "resonance", "ecocentric"] },
                        "description": "Which lenses to run. Omit to run all four (default)."
                    },
                    "metadata": {
                        "type": "object",
                        "properties": {
                            "title": { "type": "string" },
                            "context": { "type": "string", "description": "Free text: what kind of document this is, intended audience, etc. Currently unused by the deterministic lenses — reserved for a possible future optional LLM-assisted mode." }
                        }
                    }
                },
                "required": ["text"]
            }
        }
    ] })
}

pub fn initialize_response() -> Value {
    json!({
        "protocolVersion": "2024-11-05",
        "capabilities": { "tools": {} },
        "serverInfo": {
            "name": "laura-mcp",
            "version": env!("CARGO_PKG_VERSION"),
            "description": "Deterministic, local review server grounded in Laura Serna Gaviria's Human-AI Co-Evolution framework. See README for full attribution.",
            "email": "contact@rfi-irfos.com",
            "url": "https://rfi-irfos.com"
        }
    })
}

pub fn handle_request(req: RpcRequest) -> RpcResponse {
    let id = req.id.unwrap_or(Value::Null);
    let params = req.params.unwrap_or(Value::Object(Default::default()));

    match req.method.as_str() {
        "initialize" => RpcResponse::ok(id, initialize_response()),
        "notifications/initialized" => RpcResponse::ok(id, json!({})),
        "tools/list" => RpcResponse::ok(id, tools_list()),
        "tools/call" => {
            let tool_name = match params["name"].as_str() {
                Some(n) => n.to_string(),
                None => return RpcResponse::err(id, -32602, "missing tool name"),
            };
            let tool_params = &params["arguments"];
            match dispatch_tool(&tool_name, tool_params) {
                Ok(result) => RpcResponse::ok(
                    id,
                    json!({ "content": [{ "type": "text", "text": serde_json::to_string_pretty(&result).unwrap_or_default() }] }),
                ),
                Err(e) => RpcResponse::err(id, -32000, e),
            }
        }
        other => RpcResponse::err(id, -32601, format!("method not found: {other}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn req(method: &str, params: Option<Value>) -> RpcRequest {
        RpcRequest { jsonrpc: "2.0".into(), id: Some(json!(1)), method: method.to_string(), params }
    }

    #[test]
    fn initialize_returns_server_info() {
        let resp = handle_request(req("initialize", None));
        assert!(resp.error.is_none());
        assert!(resp.result.unwrap()["serverInfo"]["name"] == "laura-mcp");
    }

    #[test]
    fn tools_call_review_plan_works() {
        let resp = handle_request(req(
            "tools/call",
            Some(json!({ "name": "review_plan", "arguments": { "text": "# Goals\nShip fast.\n\n# Risks\nMight break." } })),
        ));
        assert!(resp.error.is_none());
    }

    #[test]
    fn unknown_method_errors() {
        let resp = handle_request(req("nonexistent", None));
        assert!(resp.error.is_some());
    }

    #[test]
    fn tools_call_missing_name_errors() {
        let resp = handle_request(req("tools/call", Some(json!({}))));
        assert_eq!(resp.error.unwrap().code, -32602);
    }
}
