use relay_interfaces::mcp;

#[derive(Debug, PartialEq, Eq)]
pub enum Dispatch {
    Discover,
    ToolsList,
    ToolsCall,
    ResourcesList,
    ResourcesRead,
    TasksGet,
    TasksUpdate,
    TasksCancel,
    AgentSessionStart,
    AgentPreStop,
    AgentSubagentStop,
    ActivityConfigure,
    ActivityStatus,
    TaskCompleted,
    Unknown(String),
}

pub fn dispatch(request: &mcp::Request) -> Dispatch {
    match request.method.as_str() {
        "server/discover" => Dispatch::Discover,
        "tools/list" => Dispatch::ToolsList,
        "tools/call" => Dispatch::ToolsCall,
        "resources/list" => Dispatch::ResourcesList,
        "resources/read" => Dispatch::ResourcesRead,
        "tasks/get" => Dispatch::TasksGet,
        "tasks/update" => Dispatch::TasksUpdate,
        "tasks/cancel" => Dispatch::TasksCancel,
        "agent/session_start" => Dispatch::AgentSessionStart,
        "agent/pre_stop" => Dispatch::AgentPreStop,
        "agent/subagent_stop" => Dispatch::AgentSubagentStop,
        "server/activity_configure" => Dispatch::ActivityConfigure,
        "server/activity_status" => Dispatch::ActivityStatus,
        "server/task_completed" => Dispatch::TaskCompleted,
        other => Dispatch::Unknown(other.to_owned()),
    }
}
