use super::mcp;

#[derive(Debug, PartialEq, Eq)]
pub enum Dispatch {
    Discover,
    ToolsList,
    ToolsCall,
    Unknown(String),
}

pub fn dispatch(request: &mcp::Request) -> Dispatch {
    match request.method.as_str() {
        "server/discover" => Dispatch::Discover,
        "tools/list" => Dispatch::ToolsList,
        "tools/call" => Dispatch::ToolsCall,
        other => Dispatch::Unknown(other.to_owned()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn request(method: &str) -> mcp::Request {
        mcp::parse_request(&json!({"jsonrpc":"2.0","id":1,"method":method})).unwrap()
    }

    #[test]
    fn dispatches_supported_methods_and_unknown_errors() {
        assert_eq!(dispatch(&request("server/discover")), Dispatch::Discover);
        assert_eq!(dispatch(&request("tools/list")), Dispatch::ToolsList);
        assert_eq!(dispatch(&request("tools/call")), Dispatch::ToolsCall);
        assert_eq!(dispatch(&request("nope")), Dispatch::Unknown("nope".into()));
    }
}
