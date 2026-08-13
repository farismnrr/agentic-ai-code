use relay_interfaces::mcp;

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
