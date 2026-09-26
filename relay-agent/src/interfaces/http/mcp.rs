use axum::{
    body::Bytes,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde_json::{json, Value};

use super::RelayHttpState;

mod auth;
mod protocol;
mod results;

pub async fn protected_resource(State(state): State<RelayHttpState>) -> Response {
    Json(state.mcp_resource.clone()).into_response()
}

pub async fn protected_resource_root(State(state): State<RelayHttpState>) -> Response {
    Json(state.mcp_resource.clone()).into_response()
}

pub async fn post(
    State(state): State<RelayHttpState>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let principal = match auth::authenticate(&state, &headers) {
        Ok(principal) => principal,
        Err(response) => return response,
    };

    if !protocol::content_type_is_json(&headers) {
        return protocol::rpc_error_response(
            StatusCode::BAD_REQUEST,
            None,
            -32600,
            "Invalid Request",
            None,
        );
    }

    let request: protocol::JsonRpcRequest = match serde_json::from_slice(&body) {
        Ok(request) => request,
        Err(_) => {
            return protocol::rpc_error_response(
                StatusCode::BAD_REQUEST,
                None,
                -32700,
                "Parse error",
                None,
            )
        }
    };

    if request.jsonrpc != "2.0" {
        return protocol::rpc_error_response(
            StatusCode::BAD_REQUEST,
            request.id,
            -32600,
            "Invalid Request",
            None,
        );
    }

    if let Err(response) = protocol::validate_request(&headers, &request) {
        return response;
    }

    let Some(id) = request.id.clone() else {
        return StatusCode::ACCEPTED.into_response();
    };

    let result = match request.method.as_str() {
        "server/discover" => results::discover_result(),
        "tools/list" => results::tools_list_result(),
        "tools/call" => match results::tools_call_result(&principal, &request.params) {
            Ok(result) => result,
            Err(message) => {
                return protocol::rpc_error_response(
                    StatusCode::BAD_REQUEST,
                    Some(id),
                    -32602,
                    message,
                    None,
                )
            }
        },
        _ => {
            return protocol::rpc_error_response(
                StatusCode::NOT_FOUND,
                Some(id),
                -32601,
                "Method not found",
                None,
            )
        }
    };

    Json(json!({ "jsonrpc": "2.0", "id": id, "result": result })).into_response()
}

pub(super) fn server_meta() -> Value {
    json!({
        "io.modelcontextprotocol/serverInfo": {
            "name": "masih-awam-relay",
            "version": "0.1.0"
        }
    })
}
