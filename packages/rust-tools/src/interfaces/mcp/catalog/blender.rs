use super::{coding_security_scheme, Tool, ToolAnnotations};
use crate::core::config::{
    BLENDER_MCP_ANIMATION_PREVIEW_TIMEOUT_MS, BLENDER_MCP_DEFAULT_TIMEOUT_MS,
    BLENDER_MCP_SCREENSHOT_TIMEOUT_MS, BLENDER_MCP_SESSION_TIMEOUT_MS,
};
use serde_json::{json, Value};

pub(super) fn tools() -> Vec<Tool> {
    vec![
        session_tool(),
        inspect_tool(),
        python_api_docs_tool(),
        screenshot_tool(),
        animation_preview_tool(),
    ]
}

fn common_properties() -> Value {
    json!({
        "cwd": { "type":"string", "minLength":1, "maxLength":4096 },
        "project_id": { "type":"string", "minLength":1, "maxLength":64, "pattern":"^[A-Za-z0-9_-]+$" }
    })
}

fn properties_with(extra: Value) -> Value {
    let mut properties = common_properties();
    if let (Some(base), Some(extra)) = (properties.as_object_mut(), extra.as_object()) {
        for (key, value) in extra {
            base.insert(key.clone(), value.clone());
        }
    }
    properties
}

fn schema(extra: Value, required: &[&str]) -> Value {
    let mut required_fields = vec![json!("cwd"), json!("project_id")];
    required_fields.extend(required.iter().map(|value| json!(value)));
    json!({
        "type":"object",
        "properties":properties_with(extra),
        "required":required_fields,
        "additionalProperties":false
    })
}

fn session_tool() -> Tool {
    Tool {
        name: "blender_session",
        title: Some("Blender Session"),
        description: "Probe, explicitly start, or owner-safely stop the operator-configured Blender Lab session for one selected project. Callers cannot provide host, port, executable, command-line arguments, or process IDs. Start/stop remain bounded by the configured bridge lifecycle deadline.",
        input_schema: schema(
            json!({"action":{"type":"string","enum":["status","start","stop"]}}),
            &["action"],
        ),
        annotations: Some(privileged_mutation(false)),
        security_schemes: coding_security_scheme(),
        execution: Some(execution_timeout(BLENDER_MCP_SESSION_TIMEOUT_MS)),
    }
}

fn inspect_tool() -> Tool {
    Tool {
        name: "blender_inspect",
        title: Some("Blender Structured Inspection"),
        description: "Read bounded structured Blender state without accepting arbitrary Python. Scopes cover scene, object, mesh, UV, rig, animation, material, nodes, physics, asset, render, and character state.",
        input_schema: schema(
            json!({
                "scope":{"type":"string","enum":["scene","object","mesh","uv","rig","animation","material","nodes","physics","asset","render","character"]},
                "target":{"type":"string","minLength":1,"maxLength":256},
                "detail":{"type":"string","enum":["summary","standard","detailed"],"default":"standard"}
            }),
            &["scope"],
        ),
        annotations: Some(read_only_privileged()),
        security_schemes: coding_security_scheme(),
        execution: Some(execution_timeout(BLENDER_MCP_DEFAULT_TIMEOUT_MS)),
    }
}

fn python_api_docs_tool() -> Tool {
    Tool {
        name: "blender_python_api_docs",
        title: Some("Blender Python API Docs"),
        description: "Look up bounded version-aligned Blender Python API/manual knowledge without executing Python or granting network authority.",
        input_schema: schema(
            json!({
                "query":{"type":"string","minLength":1,"maxLength":512},
                "module":{"type":"string","minLength":1,"maxLength":128},
                "limit":{"type":"integer","minimum":1,"maximum":20,"default":8}
            }),
            &["query"],
        ),
        annotations: Some(read_only()),
        security_schemes: coding_security_scheme(),
        execution: Some(execution_timeout(BLENDER_MCP_DEFAULT_TIMEOUT_MS)),
    }
}

fn screenshot_tool() -> Tool {
    Tool {
        name: "blender_screenshot",
        title: Some("Blender Screenshot"),
        description: "Capture a bounded Blender viewport or render-result image, optionally materializing a named preview under blender/renders/preview/. This is a bounded inspection/preview path, not a final render surface.",
        input_schema: schema(
            json!({
                "source":{"type":"string","enum":["viewport","render_result"],"default":"viewport"},
                "width":{"type":"integer","minimum":64,"maximum":4096},
                "height":{"type":"integer","minimum":64,"maximum":4096},
                "save_name":safe_file_name()
            }),
            &[],
        ),
        annotations: Some(privileged_mutation(false)),
        security_schemes: coding_security_scheme(),
        execution: Some(execution_timeout(BLENDER_MCP_SCREENSHOT_TIMEOUT_MS)),
    }
}

fn animation_preview_tool() -> Tool {
    Tool {
        name: "blender_animation_preview",
        title: Some("Blender Animation Preview"),
        description: "Render a bounded sampled frame sequence for visual review. Preview images are contained under blender/renders/preview/, and small previews may be returned inline for direct agent inspection.",
        input_schema: schema(
            json!({
                "start_frame":{"type":"integer"},
                "end_frame":{"type":"integer"},
                "step":{"type":"integer","minimum":1,"maximum":10000,"default":1},
                "max_frames":{"type":"integer","minimum":1,"maximum":240,"default":48},
                "width":{"type":"integer","minimum":64,"maximum":4096,"default":640},
                "height":{"type":"integer","minimum":64,"maximum":4096,"default":360},
                "save_name":safe_file_name()
            }),
            &["start_frame", "end_frame"],
        ),
        annotations: Some(privileged_mutation(false)),
        security_schemes: coding_security_scheme(),
        execution: Some(execution_timeout(BLENDER_MCP_ANIMATION_PREVIEW_TIMEOUT_MS)),
    }
}

fn execution_timeout(timeout_ms: u64) -> Value {
    json!({ "timeoutMs": timeout_ms })
}

fn safe_file_name() -> Value {
    json!({
        "type":"string",
        "minLength":1,
        "maxLength":180,
        "pattern":"^[A-Za-z0-9][A-Za-z0-9._ -]{0,179}$"
    })
}

fn read_only() -> ToolAnnotations {
    ToolAnnotations {
        read_only_hint: true,
        destructive_hint: false,
        idempotent_hint: true,
        open_world_hint: false,
    }
}

fn read_only_privileged() -> ToolAnnotations {
    read_only()
}

fn privileged_mutation(open_world: bool) -> ToolAnnotations {
    ToolAnnotations {
        read_only_hint: false,
        destructive_hint: false,
        idempotent_hint: false,
        open_world_hint: open_world,
    }
}
