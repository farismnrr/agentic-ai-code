use super::contracts::{validate_id, validate_spec, CREATIVE_SCHEMA_VERSION};
use super::registry::{self, ExecutionBindingDescriptor};
use crate::core::config::ServerConfig;
use crate::core::error::McpError;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::HashSet;

pub const COMPILER_VERSION: &str = "creative.semantic.v1";
const MAX_REFERENCES: usize = 32;
const MAX_CHANGED_FIELDS: usize = 32;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct SemanticReference {
    pub asset_id: String,
    pub role: String,
    pub order: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct SemanticRequestSpec {
    pub schema_version: u32,
    #[serde(default)]
    pub prompt: Option<String>,
    #[serde(default)]
    pub references: Vec<SemanticReference>,
    #[serde(default)]
    pub asset_id: Option<String>,
    #[serde(default)]
    pub mask_asset_id: Option<String>,
    #[serde(default)]
    pub element_id: Option<String>,
    #[serde(default)]
    pub width: Option<u32>,
    #[serde(default)]
    pub height: Option<u32>,
    #[serde(default)]
    pub batch_count: Option<u32>,
    #[serde(default)]
    pub duration_ms: Option<u64>,
    #[serde(default)]
    pub binding_extensions: Map<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CompiledSemanticSpec {
    pub compiler_version: String,
    pub capability_id: String,
    pub execution_binding_id: String,
    pub execution_binding_version: String,
    pub parameters: Value,
    #[serde(default)]
    pub changed_fields: Vec<String>,
}

pub fn compile(
    config: &ServerConfig,
    capability_id: &str,
    binding_id: &str,
    spec: &Value,
    changed_fields: &[String],
) -> Result<CompiledSemanticSpec, McpError> {
    validate_spec(spec)?;
    let parsed: SemanticRequestSpec = serde_json::from_value(spec.clone())
        .map_err(|_| McpError::InvalidRequest("creative semantic spec is invalid".into()))?;
    validate_semantic_spec(&parsed)?;
    let binding = registry::validate_binding_selection(config, capability_id, Some(binding_id))?
        .ok_or_else(|| {
            McpError::InvalidRequest("creative semantic compiler requires a binding".into())
        })?;
    let changed_fields = validate_changed_fields(changed_fields)?;
    let parameters = compile_parameters(&parsed, &binding)?;
    Ok(CompiledSemanticSpec {
        compiler_version: COMPILER_VERSION.into(),
        capability_id: capability_id.into(),
        execution_binding_id: binding.binding_id,
        execution_binding_version: binding.binding_version,
        parameters,
        changed_fields,
    })
}

fn validate_semantic_spec(spec: &SemanticRequestSpec) -> Result<(), McpError> {
    if spec.schema_version != CREATIVE_SCHEMA_VERSION {
        return Err(McpError::InvalidRequest(
            "creative semantic spec schema version is unsupported".into(),
        ));
    }
    if spec.prompt.as_deref().is_some_and(|value| {
        value.is_empty() || value.len() > 8192 || value.chars().any(char::is_control)
    }) {
        return Err(McpError::InvalidRequest(
            "creative semantic prompt exceeds allowed bounds".into(),
        ));
    }
    if spec.references.len() > MAX_REFERENCES {
        return Err(McpError::InvalidRequest(
            "creative semantic reference count exceeds maximum".into(),
        ));
    }
    let mut asset_ids = HashSet::new();
    let mut orders = HashSet::new();
    for reference in &spec.references {
        validate_id(&reference.asset_id, "reference asset id")?;
        if reference.role.is_empty()
            || reference.role.len() > 128
            || reference.role.chars().any(char::is_control)
            || !asset_ids.insert(reference.asset_id.as_str())
            || !orders.insert(reference.order)
        {
            return Err(McpError::InvalidRequest(
                "creative semantic reference is invalid or duplicated".into(),
            ));
        }
    }
    for (value, label) in [
        (spec.asset_id.as_deref(), "asset_id"),
        (spec.mask_asset_id.as_deref(), "mask_asset_id"),
        (spec.element_id.as_deref(), "element_id"),
    ] {
        if let Some(value) = value {
            validate_id(value, label)?;
        }
    }
    for dimension in [spec.width, spec.height].into_iter().flatten() {
        if dimension == 0 || dimension > 16_384 {
            return Err(McpError::InvalidRequest(
                "creative semantic dimensions exceed allowed bounds".into(),
            ));
        }
    }
    if spec
        .batch_count
        .is_some_and(|value| value == 0 || value > 128)
        || spec
            .duration_ms
            .is_some_and(|value| value == 0 || value > 86_400_000)
    {
        return Err(McpError::InvalidRequest(
            "creative semantic request dimensions exceed allowed bounds".into(),
        ));
    }
    if spec.binding_extensions.len() > 32 {
        return Err(McpError::InvalidRequest(
            "creative semantic binding extension count exceeds maximum".into(),
        ));
    }
    for key in spec.binding_extensions.keys() {
        validate_id(key, "binding extension id")?;
    }
    Ok(())
}

fn validate_changed_fields(fields: &[String]) -> Result<Vec<String>, McpError> {
    if fields.len() > MAX_CHANGED_FIELDS {
        return Err(McpError::InvalidRequest(
            "creative changed-fields count exceeds maximum".into(),
        ));
    }
    const ALLOWED: &[&str] = &[
        "prompt",
        "references",
        "asset_id",
        "mask_asset_id",
        "element_id",
        "width",
        "height",
        "batch_count",
        "duration_ms",
        "binding_extensions",
    ];
    let mut seen = HashSet::new();
    let mut result = Vec::with_capacity(fields.len());
    for field in fields {
        if !ALLOWED.contains(&field.as_str()) || !seen.insert(field.as_str()) {
            return Err(McpError::InvalidRequest(
                "creative changed field is unsupported or duplicated".into(),
            ));
        }
        result.push(field.clone());
    }
    result.sort();
    Ok(result)
}

fn compile_parameters(
    spec: &SemanticRequestSpec,
    binding: &ExecutionBindingDescriptor,
) -> Result<Value, McpError> {
    let mut parameters = Map::new();
    insert_option(
        &mut parameters,
        "prompt",
        spec.prompt.clone().map(Value::String),
    );
    insert_option(
        &mut parameters,
        "asset_id",
        spec.asset_id.clone().map(Value::String),
    );
    insert_option(
        &mut parameters,
        "mask_asset_id",
        spec.mask_asset_id.clone().map(Value::String),
    );
    insert_option(
        &mut parameters,
        "element_id",
        spec.element_id.clone().map(Value::String),
    );
    insert_option(&mut parameters, "width", spec.width.map(Value::from));
    insert_option(&mut parameters, "height", spec.height.map(Value::from));
    insert_option(
        &mut parameters,
        "batch_count",
        spec.batch_count.map(Value::from),
    );
    insert_option(
        &mut parameters,
        "duration_ms",
        spec.duration_ms.map(Value::from),
    );

    if !spec.references.is_empty() {
        let mut references = spec.references.clone();
        references.sort_by_key(|reference| reference.order);
        parameters.insert(
            "reference_asset_ids".into(),
            Value::Array(
                references
                    .iter()
                    .map(|reference| Value::String(reference.asset_id.clone()))
                    .collect(),
            ),
        );
        parameters.insert(
            "reference_roles".into(),
            Value::Array(
                references
                    .iter()
                    .map(|reference| {
                        serde_json::json!({
                            "asset_id": reference.asset_id,
                            "role": reference.role,
                            "order": reference.order,
                        })
                    })
                    .collect(),
            ),
        );
    }

    if let Some(extension) = spec.binding_extensions.get(&binding.binding_id) {
        let validator = jsonschema::validator_for(&binding.extension_schema).map_err(|_| {
            McpError::Internal("creative binding extension schema is invalid".into())
        })?;
        if validator.iter_errors(extension).next().is_some() {
            return Err(McpError::InvalidRequest(
                "creative binding extension does not match selected binding schema".into(),
            ));
        }
        parameters.insert("binding_extensions".into(), extension.clone());
    }
    Ok(Value::Object(parameters))
}

fn insert_option(map: &mut Map<String, Value>, key: &str, value: Option<Value>) {
    if let Some(value) = value {
        map.insert(key.into(), value);
    }
}
