import sys

def patch_config():
    path = "packages/rust-tools/src/relay_agent/config.rs"
    with open(path, "r") as f:
        code = f.read()

    code = code.replace(
        "pub enum Command {",
        "#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum, Deserialize, Serialize)]\n"
        "pub enum SecurityMode {\n    #[serde(rename = \"local\")]\n    Local,\n    #[serde(rename = \"remote\")]\n    Remote,\n}\n\n"
        "#[derive(Subcommand, Debug)]\n"
        "pub enum Command {"
    )
    code = code.replace(
        "pub port: u16,\n\n    /// Default",
        "pub port: u16,\n\n    #[arg(long, value_enum, env = \"RELAY_AGENT_MODE\", default_value = \"local\")]\n    pub mode: SecurityMode,\n\n    /// Default"
    )
    code = code.replace(
        "pub port: u16,\n    pub dir:",
        "pub port: u16,\n    pub mode: SecurityMode,\n    pub dir:"
    )
    code = code.replace(
        "port: DEFAULT_PORT,\n            dir: None,",
        "port: DEFAULT_PORT,\n            mode: SecurityMode::Local,\n            dir: None,"
    )
    code = code.replace(
        "port: cli.port,\n            dir:",
        "port: cli.port,\n            mode: cli.mode,\n            dir:"
    )

    with open(path, "w") as f:
        f.write(code)

def patch_main():
    path = "packages/rust-tools/src/bin/relay-agent.rs"
    with open(path, "r") as f:
        code = f.read()
    
    code = code.replace(
        "let addr = SocketAddr::from(([127, 0, 0, 1], config.port));",
        "let addr = match config.mode {\n"
        "        rust_tools::relay_agent::config::SecurityMode::Local => {\n"
        "            SocketAddr::from(([127, 0, 0, 1], config.port))\n"
        "        }\n"
        "        rust_tools::relay_agent::config::SecurityMode::Remote => {\n"
        "            SocketAddr::from(([0, 0, 0, 0], config.port))\n"
        "        }\n"
        "    };"
    )
    
    with open(path, "w") as f:
        f.write(code)

def patch_transport():
    path = "packages/rust-tools/src/relay_agent/transport.rs"
    with open(path, "r") as f:
        code = f.read()
    
    patch = """
    if let super::config::SecurityMode::Remote = state.config.mode {
        let is_https = req.headers().get("x-forwarded-proto").map(|v| v.as_bytes()) == Some(b"https")
            || req.uri().scheme_str() == Some("https");
        
        if !is_https {
            return (
                StatusCode::FORBIDDEN,
                Json(ErrorResponse::new(None, &McpError::InvalidRequest("Remote mode requires HTTPS (or proxy header)".into())))
            ).into_response();
        }
    }
"""
    code = code.replace(
        "let mut auth_ctx = AuthContext::default();",
        "let mut auth_ctx = AuthContext::default();\n" + patch
    )
    with open(path, "w") as f:
        f.write(code)

patch_config()
patch_main()
patch_transport()
print("Patched correctly.")
