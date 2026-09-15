use super::{coding_security_scheme, Tool, ToolAnnotations};
use serde_json::{json, Value};

pub(super) fn tools() -> Vec<Tool> {
    vec![
        session_tool(),
        inspect_tool(),
        python_api_docs_tool(),
        execute_python_tool(),
        screenshot_tool(),
        animation_preview_tool(),
        render_tool(),
        asset_import_tool(),
        asset_export_tool(),
        checkpoint_create_tool(),
        checkpoint_restore_tool(),
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
        description: "Probe, explicitly start, or owner-safely stop the operator-configured Blender Lab session for one selected project. Callers cannot provide host, port, executable, command-line arguments, or process IDs.",
        input_schema: schema(
            json!({"action":{"type":"string","enum":["status","start","stop"]}}),
            &["action"],
        ),
        annotations: Some(privileged_mutation(false)),
        security_schemes: coding_security_scheme(),
        execution: None,
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
        execution: None,
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
        execution: None,
    }
}

fn execute_python_tool() -> Tool {
    Tool {
        name: "blender_execute_python",
        title: Some("Blender Privileged Python"),
        description: "Execute bounded caller-authored Python inside the attached Blender host. This is privileged host-user execution, not a sandbox; production paths remain constrained by the surrounding Blender workflow contract.",
        input_schema: schema(
            json!({
                "code":{"type":"string","minLength":1,"maxLength":262144},
                "result_mode":{"type":"string","enum":["json","text"],"default":"json"}
            }),
            &["code"],
        ),
        annotations: Some(privileged_mutation(true)),
        security_schemes: coding_security_scheme(),
        execution: None,
    }
}

fn screenshot_tool() -> Tool {
    Tool {
        name: "blender_screenshot",
        title: Some("Blender Screenshot"),
        description: "Capture a bounded Blender viewport or render-result image, optionally materializing a named preview under blender/renders/preview/. Arbitrary destination paths are not accepted.",
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
        execution: None,
    }
}

fn animation_preview_tool() -> Tool {
    Tool {
        name: "blender_animation_preview",
        title: Some("Blender Animation Preview"),
        description: "Sample a bounded frame range for temporal review and optionally materialize the preview beneath blender/renders/preview/ or blender/animations/. No arbitrary output path is accepted.",
        input_schema: schema(
            json!({
                "start_frame":{"type":"integer","minimum":-1000000,"maximum":1000000},
                "end_frame":{"type":"integer","minimum":-1000000,"maximum":1000000},
                "step":{"type":"integer","minimum":1,"maximum":120,"default":1},
                "max_frames":{"type":"integer","minimum":1,"maximum":240,"default":48},
                "width":{"type":"integer","minimum":64,"maximum":4096},
                "height":{"type":"integer","minimum":64,"maximum":4096},
                "save_name":safe_file_name()
            }),
            &["start_frame","end_frame"],
        ),
        annotations: Some(privileged_mutation(false)),
        security_schemes: coding_security_scheme(),
        execution: None,
    }
}

fn render_tool() -> Tool {
    Tool {
        name: "blender_render",
        title: Some("Blender Render"),
        description: "Render a still or bounded animation into the canonical project Blender preview/final directories. Output scope and leaf name are explicit; absolute paths, URLs, and traversal are not accepted.",
        input_schema: schema(
            json!({
                "mode":{"type":"string","enum":["still","animation"]},
                "output_scope":{"type":"string","enum":["preview","final"]},
                "file_name":safe_file_name(),
                "start_frame":{"type":"integer","minimum":-1000000,"maximum":1000000},
                "end_frame":{"type":"integer","minimum":-1000000,"maximum":1000000}
            }),
            &["mode","output_scope","file_name"],
        ),
        annotations: Some(privileged_mutation(false)),
        security_schemes: coding_security_scheme(),
        execution: None,
    }
}

fn asset_import_tool() -> Tool {
    Tool {
        name: "blender_asset_import",
        title: Some("Blender Asset Import"),
        description: "Materialize and import one already-contained Creative Asset into blender/references/ or blender/assets/. Direct URLs and arbitrary host paths are intentionally absent from the schema.",
        input_schema: schema(
            json!({
                "asset_id":{"type":"string","minLength":1,"maxLength":64,"pattern":"^[A-Za-z0-9_-]+$"},
                "purpose":{"type":"string","enum":["reference","asset"]},
                "target_name":safe_file_name(),
                "collection":{"type":"string","minLength":1,"maxLength":128}
            }),
            &["asset_id","purpose"],
        ),
        annotations: Some(privileged_mutation(false)),
        security_schemes: coding_security_scheme(),
        execution: None,
    }
}

fn asset_export_tool() -> Tool {
    Tool {
        name: "blender_asset_export",
        title: Some("Blender Asset Export"),
        description: "Export selected Blender objects/collections beneath blender/exports/. The caller selects a reviewed format and leaf file name, never an arbitrary destination.",
        input_schema: schema(
            json!({
                "selection":{"type":"array","minItems":1,"maxItems":128,"uniqueItems":true,"items":{"type":"string","minLength":1,"maxLength":256}},
                "format":{"type":"string","enum":["glb","gltf","fbx","obj","usd","usdz"]},
                "output_scope":{"type":"string","enum":["export","animation"],"default":"export"},
                "file_name":safe_file_name()
            }),
            &["selection","format","file_name"],
        ),
        annotations: Some(privileged_mutation(false)),
        security_schemes: coding_security_scheme(),
        execution: None,
    }
}

fn checkpoint_create_tool() -> Tool {
    Tool {
        name: "blender_checkpoint_create",
        title: Some("Blender Checkpoint Create"),
        description: "Create a bounded recovery checkpoint beneath blender/checkpoints/ with relay-owned identity. Caller labels are metadata, not paths.",
        input_schema: schema(
            json!({"label":{"type":"string","minLength":1,"maxLength":128}}),
            &[],
        ),
        annotations: Some(privileged_mutation(false)),
        security_schemes: coding_security_scheme(),
        execution: None,
    }
}

fn checkpoint_restore_tool() -> Tool {
    Tool {
        name: "blender_checkpoint_restore",
        title: Some("Blender Checkpoint Restore"),
        description: "Replace the live Blender scene from one relay-created project checkpoint. This is destructive and cannot target arbitrary files.",
        input_schema: schema(
            json!({"checkpoint_id":{"type":"string","minLength":1,"maxLength":64,"pattern":"^[A-Za-z0-9_-]+$"}}),
            &["checkpoint_id"],
        ),
        annotations: Some(destructive_privileged()),
        security_schemes: coding_security_scheme(),
        execution: None,
    }
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

fn destructive_privileged() -> ToolAnnotations {
    ToolAnnotations {
        read_only_hint: false,
        destructive_hint: true,
        idempotent_hint: false,
        open_world_hint: false,
    }
}
