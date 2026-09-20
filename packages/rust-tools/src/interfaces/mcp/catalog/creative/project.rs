use super::*;

pub(super) fn project_tool() -> Tool {
    Tool {
        name: "creative_project",
        title: Some("Creative Project State"),
        description: "Create, read, or list versioned workspace-contained Creative Projects. Projects own reusable Elements, Assets, Scene/Game state, graph/job identities, QA, and provenance without provider/model identities as source of truth.",
        input_schema: json!({
            "type": "object",
            "properties": {
                "action": { "type": "string", "enum": ["create", "get", "list", "scene_board_put", "scene_board_get", "scene_put", "scene_get", "game_put", "game_get", "qa_add", "qa_list", "room_create", "room_join", "room_update", "room_get", "room_leave"] },
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
                "target": { "type": "object", "maxProperties": 8 },
                "board_id": { "type": "string", "minLength": 1, "maxLength": 64 },
                "scene_id": { "type": "string", "minLength": 1, "maxLength": 64 },
                "game_id": { "type": "string", "minLength": 1, "maxLength": 64 },
                "scene_board": { "type": "object", "maxProperties": 16 },
                "scene": { "type": "object", "maxProperties": 32 },
                "game": { "type": "object", "maxProperties": 32 },
                "domain": { "type": "string", "minLength": 1, "maxLength": 64 },
                "severity": { "type": "string", "enum": ["hard_fail", "soft_finding", "not_inspected"] },
                "subject_id": { "type": "string", "minLength": 1, "maxLength": 128 },
                "message": { "type": "string", "minLength": 1, "maxLength": 4096 },
                "source_revision_id": { "type": "string", "minLength": 1, "maxLength": 64 },
                "asset_id": { "type": "string", "minLength": 1, "maxLength": 64 },
                "evaluator_binding_id": { "type": "string", "minLength": 1, "maxLength": 64 },
                "evidence": { "type": "object", "maxProperties": 64 },
                "room_id": { "type": "string", "minLength": 1, "maxLength": 64 },
                "member_id": { "type": "string", "minLength": 1, "maxLength": 64 },
                "expected_revision": { "type": "integer", "minimum": 0 },
                "shared_state": { "type": "object", "maxProperties": 128 }
            },
            "allOf": [
                {
                    "if": { "properties": { "action": { "const": "create" } }, "required": ["action"] },
                    "then": { "required": ["title", "intent", "tracks"] }
                },
                {
                    "if": { "properties": { "action": { "const": "get" } }, "required": ["action"] },
                    "then": { "required": ["project_id"] }
                },
                {
                    "if": { "properties": { "action": { "enum": ["scene_board_put", "scene_put", "game_put", "qa_add", "qa_list"] } }, "required": ["action"] },
                    "then": { "required": ["project_id"] }
                },
                {
                    "if": { "properties": { "action": { "const": "scene_board_put" } }, "required": ["action"] },
                    "then": { "required": ["scene_board"] }
                },
                {
                    "if": { "properties": { "action": { "const": "scene_board_get" } }, "required": ["action"] },
                    "then": { "required": ["project_id", "board_id"] }
                },
                {
                    "if": { "properties": { "action": { "const": "scene_put" } }, "required": ["action"] },
                    "then": { "required": ["scene"] }
                },
                {
                    "if": { "properties": { "action": { "const": "scene_get" } }, "required": ["action"] },
                    "then": { "required": ["project_id", "scene_id"] }
                },
                {
                    "if": { "properties": { "action": { "const": "game_put" } }, "required": ["action"] },
                    "then": { "required": ["game"] }
                },
                {
                    "if": { "properties": { "action": { "const": "game_get" } }, "required": ["action"] },
                    "then": { "required": ["project_id", "game_id"] }
                },
                {
                    "if": { "properties": { "action": { "const": "qa_add" } }, "required": ["action"] },
                    "then": { "required": ["project_id", "domain", "severity", "subject_id", "message"] }
                },
                {
                    "if": { "properties": { "action": { "const": "room_create" } }, "required": ["action"] },
                    "then": { "required": ["project_id", "game_id", "member_id"] }
                },
                {
                    "if": { "properties": { "action": { "enum": ["room_join", "room_leave"] } }, "required": ["action"] },
                    "then": { "required": ["project_id", "game_id", "room_id", "member_id"] }
                },
                {
                    "if": { "properties": { "action": { "const": "room_update" } }, "required": ["action"] },
                    "then": { "required": ["project_id", "game_id", "room_id", "expected_revision", "shared_state"] }
                },
                {
                    "if": { "properties": { "action": { "const": "room_get" } }, "required": ["action"] },
                    "then": { "required": ["project_id", "game_id", "room_id"] }
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
