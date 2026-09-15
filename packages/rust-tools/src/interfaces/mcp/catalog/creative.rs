use super::{coding_security_scheme, Tool, ToolAnnotations};
use serde_json::json;

pub(super) fn tools() -> Vec<Tool> {
    vec![
        status_tool(),
        catalog_tool(),
        project_tool(),
        element_tool(),
        asset_tool(),
        graph_tool(),
        job_tool(),
    ]
}

fn status_tool() -> Tool {
    Tool {
        name: "creative_status",
        title: Some("Creative Production Status"),
        description: "Inspect whether the first-party creative production capability is enabled, its schema version, and the operator activation hint. This status tool remains visible while the capability group is disabled.",
        input_schema: json!({
            "type": "object",
            "properties": {},
            "additionalProperties": false
        }),
        annotations: Some(read_only()),
        security_schemes: coding_security_scheme(),
        execution: None,
    }
}

fn catalog_tool() -> Tool {
    Tool {
        name: "creative_catalog",
        title: Some("Creative Capability Catalog"),
        description: "Discover semantic creative capabilities, compatible opaque execution bindings, or workflows as separate catalogs. Discovery never ranks, defaults, or auto-selects providers/models.",
        input_schema: json!({
            "type": "object",
            "properties": {
                "catalog": {
                    "type": "string",
                    "enum": ["capabilities", "execution_bindings", "workflows"]
                },
                "id": { "type": "string", "minLength": 1, "maxLength": 128 }
            },
            "required": ["catalog"],
            "additionalProperties": false
        }),
        annotations: Some(read_only()),
        security_schemes: coding_security_scheme(),
        execution: None,
    }
}

fn project_tool() -> Tool {
    Tool {
        name: "creative_project",
        title: Some("Creative Project State"),
        description: "Create, read, or list versioned workspace-contained Creative Projects. Projects own reusable Elements, Assets, Scene/Game/Audio state, graph/job identities, QA, and provenance without provider/model identities as source of truth.",
        input_schema: json!({
            "type": "object",
            "properties": {
                "action": { "type": "string", "enum": ["create", "get", "list"] },
                "cwd": { "type": "string", "maxLength": 4096 },
                "project_id": { "type": "string", "minLength": 1, "maxLength": 64 },
                "title": { "type": "string", "minLength": 1, "maxLength": 200 },
                "intent": { "type": "string", "minLength": 1, "maxLength": 4096 },
                "tracks": {
                    "type": "array",
                    "minItems": 1,
                    "maxItems": 3,
                    "uniqueItems": true,
                    "items": { "type": "string", "enum": ["scene", "anime", "game"] }
                },
                "target": { "type": "object", "maxProperties": 8 }
            },
            "allOf": [
                {
                    "if": { "properties": { "action": { "const": "create" } }, "required": ["action"] },
                    "then": { "required": ["title", "intent", "tracks"] }
                },
                {
                    "if": { "properties": { "action": { "const": "get" } }, "required": ["action"] },
                    "then": { "required": ["project_id"] }
                }
            ],
            "required": ["action"],
            "additionalProperties": false
        }),
        annotations: Some(workspace_mutation()),
        security_schemes: coding_security_scheme(),
        execution: None,
    }
}

fn element_tool() -> Tool {
    Tool {
        name: "creative_element",
        title: Some("Creative Element Library"),
        description: "Create candidate Element revisions, explicitly promote an accepted revision, or read reusable Character/Location/Prop/Style/Voice/Media/3D/Animation Elements. Revisions preserve authoritative versus interpreted/generated provenance.",
        input_schema: json!({
            "type": "object",
            "properties": {
                "action": { "type": "string", "enum": ["create_revision", "promote", "get", "list"] },
                "cwd": { "type": "string", "maxLength": 4096 },
                "project_id": { "type": "string", "minLength": 1, "maxLength": 64 },
                "element_id": { "type": "string", "minLength": 1, "maxLength": 64 },
                "revision_id": { "type": "string", "minLength": 1, "maxLength": 64 },
                "kind": {
                    "type": "string",
                    "enum": ["character", "location", "prop", "style", "audio_voice", "media", "asset3d", "animation_clip"]
                },
                "name": { "type": "string", "minLength": 1, "maxLength": 200 },
                "authority": {
                    "type": "string",
                    "enum": ["authoritative", "interpreted", "generated", "imported"]
                },
                "reference_asset_ids": {
                    "type": "array",
                    "maxItems": 64,
                    "items": { "type": "string", "minLength": 1, "maxLength": 64 }
                },
                "spec": { "type": "object", "maxProperties": 128 }
            },
            "allOf": [
                {
                    "if": { "properties": { "action": { "const": "create_revision" } }, "required": ["action"] },
                    "then": {
                        "required": ["authority"],
                        "anyOf": [
                            { "required": ["element_id"] },
                            { "required": ["kind", "name"] }
                        ]
                    }
                },
                {
                    "if": { "properties": { "action": { "const": "promote" } }, "required": ["action"] },
                    "then": { "required": ["element_id", "revision_id"] }
                },
                {
                    "if": { "properties": { "action": { "const": "get" } }, "required": ["action"] },
                    "then": { "required": ["element_id"] }
                }
            ],
            "required": ["action", "project_id"],
            "additionalProperties": false
        }),
        annotations: Some(workspace_mutation()),
        security_schemes: coding_security_scheme(),
        execution: None,
    }
}

fn asset_tool() -> Tool {
    Tool {
        name: "creative_asset",
        title: Some("Creative Asset Registry"),
        description: "Register an existing contained workspace file as a candidate manual-import Asset with SHA-256 provenance and optional parent/Element lineage, explicitly promote a candidate Asset, or read project Asset records. Generated/upload/URL/engine provenance is reserved for the owning first-party ingest/executor path and cannot be claimed through this generic registration primitive.",
        input_schema: json!({
            "type": "object",
            "properties": {
                "action": { "type": "string", "enum": ["register", "promote", "get", "list", "search"] },
                "cwd": { "type": "string", "maxLength": 4096 },
                "project_id": { "type": "string", "minLength": 1, "maxLength": 64 },
                "asset_id": { "type": "string", "minLength": 1, "maxLength": 64 },
                "path": { "type": "string", "minLength": 1, "maxLength": 4096 },
                "media_type": { "type": "string", "minLength": 1, "maxLength": 128 },
                "role": { "type": "string", "minLength": 1, "maxLength": 128 },
                "source": {
                    "type": "string",
                    "enum": ["conversation_upload", "mcp_upload", "url_import", "generated_asset", "manual_import"]
                },
                "source_surface": {
                    "type": "string",
                    "enum": ["mcp", "canvas", "scene", "anime", "game", "blender", "manual_import"]
                },
                "state": { "type": "string", "enum": ["candidate", "accepted", "rejected"] },
                "job_id": { "type": "string", "minLength": 1, "maxLength": 64 },
                "parent_asset_id": { "type": "string", "minLength": 1, "maxLength": 64 },
                "element_id": { "type": "string", "minLength": 1, "maxLength": 64 },
                "metadata": {
                    "type": "object",
                    "properties": {
                        "width": { "type": "integer", "minimum": 1, "maximum": 16384 },
                        "height": { "type": "integer", "minimum": 1, "maximum": 16384 },
                        "duration_ms": { "type": "integer", "minimum": 1, "maximum": 86400000 },
                        "frame_rate": { "type": "number", "minimum": 1, "maximum": 240 },
                        "language": { "type": "string", "minLength": 1, "maxLength": 32 },
                        "sample_rate_hz": { "type": "integer", "minimum": 8000, "maximum": 384000 },
                        "channels": { "type": "integer", "minimum": 1, "maximum": 32 }
                    },
                    "additionalProperties": false
                },
                "created_after_ms": { "type": "integer", "minimum": 0 },
                "created_before_ms": { "type": "integer", "minimum": 0 },
                "limit": { "type": "integer", "minimum": 1, "maximum": 200, "default": 50 }
            },
            "allOf": [
                {
                    "if": { "properties": { "action": { "const": "register" } }, "required": ["action"] },
                    "then": {
                        "required": ["path", "media_type", "role"],
                        "not": {
                            "anyOf": [
                                { "required": ["source"] },
                                { "required": ["source_surface"] },
                                { "required": ["state"] },
                                { "required": ["job_id"] },
                                { "required": ["created_after_ms"] },
                                { "required": ["created_before_ms"] },
                                { "required": ["limit"] }
                            ]
                        }
                    }
                },
                {
                    "if": {
                        "properties": { "action": { "enum": ["get", "promote"] } },
                        "required": ["action"]
                    },
                    "then": { "required": ["asset_id"] }
                }
            ],
            "required": ["action", "project_id"],
            "additionalProperties": false
        }),
        annotations: Some(workspace_mutation()),
        security_schemes: coding_security_scheme(),
        execution: None,
    }
}

fn graph_tool() -> Tool {
    Tool {
        name: "creative_graph",
        title: Some("Creative Graph Runtime"),
        description: "Validate, execute, or retrieve a caller-authored typed Creative Graph. The minimal headless executor handles reviewed control/reference nodes, preserves DAG authority boundaries, and refuses pluggable executor nodes without an explicit compatible execution_binding_id.",
        input_schema: json!({
            "type": "object",
            "properties": {
                "action": { "type": "string", "enum": ["validate", "execute", "get"] },
                "cwd": { "type": "string", "maxLength": 4096 },
                "project_id": { "type": "string", "minLength": 1, "maxLength": 64 },
                "graph_id": { "type": "string", "minLength": 1, "maxLength": 64 },
                "graph": { "type": "object", "maxProperties": 16 }
            },
            "allOf": [
                {
                    "if": {
                        "properties": { "action": { "enum": ["validate", "execute"] } },
                        "required": ["action"]
                    },
                    "then": { "required": ["graph"] }
                },
                {
                    "if": { "properties": { "action": { "const": "get" } }, "required": ["action"] },
                    "then": { "required": ["project_id", "graph_id"] }
                }
            ],
            "required": ["action"],
            "additionalProperties": false
        }),
        annotations: Some(workspace_mutation()),
        security_schemes: coding_security_scheme(),
        execution: None,
    }
}

fn job_tool() -> Tool {
    Tool {
        name: "creative_job",
        title: Some("Creative Job State"),
        description: "Read, list, or cancel durable project-owned creative job records. Current headless graph runs persist terminal job state; later executor-backed jobs reuse this contract rather than creating a second job system.",
        input_schema: json!({
            "type": "object",
            "properties": {
                "action": { "type": "string", "enum": ["get", "list", "cancel"] },
                "cwd": { "type": "string", "maxLength": 4096 },
                "project_id": { "type": "string", "minLength": 1, "maxLength": 64 },
                "job_id": { "type": "string", "minLength": 1, "maxLength": 64 }
            },
            "allOf": [
                {
                    "if": {
                        "properties": { "action": { "enum": ["get", "cancel"] } },
                        "required": ["action"]
                    },
                    "then": { "required": ["job_id"] }
                }
            ],
            "required": ["action", "project_id"],
            "additionalProperties": false
        }),
        annotations: Some(workspace_mutation()),
        security_schemes: coding_security_scheme(),
        execution: None,
    }
}

fn read_only() -> ToolAnnotations {
    ToolAnnotations {
        read_only_hint: true,
        destructive_hint: false,
        idempotent_hint: true,
        open_world_hint: false,
    }
}

fn workspace_mutation() -> ToolAnnotations {
    ToolAnnotations {
        read_only_hint: false,
        destructive_hint: false,
        idempotent_hint: false,
        open_world_hint: false,
    }
}
