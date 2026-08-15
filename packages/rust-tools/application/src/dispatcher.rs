use relay_interfaces::mcp;

#[derive(Debug, PartialEq, Eq)]
pub enum Dispatch {
    Discover,
    ToolsList,
    ToolsCall,
    TasksGet,
    TasksUpdate,
    TasksCancel,
    Unknown(String),
}

pub fn dispatch(request: &mcp::Request) -> Dispatch {
    match request.method.as_str() {
        "server/discover" => Dispatch::Discover,
        "tools/list" => Dispatch::ToolsList,
        "tools/call" => Dispatch::ToolsCall,
        "tasks/get" => Dispatch::TasksGet,
        "tasks/update" => Dispatch::TasksUpdate,
        "tasks/cancel" => Dispatch::TasksCancel,
        other => Dispatch::Unknown(other.to_owned()),
    }
}
