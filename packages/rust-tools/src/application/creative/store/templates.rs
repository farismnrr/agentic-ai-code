use super::io::{read_json, write_json};
use super::{load_project, state_exists};
use crate::application::creative::contracts::{validate_id, CREATIVE_SCHEMA_VERSION};
use crate::application::creative::graph::{validate_graph_template, CreativeGraphTemplate};
use crate::core::config::ServerConfig;
use crate::core::error::McpError;
use serde::{Deserialize, Serialize};

const MAX_TEMPLATES_PER_PROJECT: usize = 256;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TemplateIndex {
    schema_version: u32,
    template_ids: Vec<String>,
}

pub fn store_template(
    cwd: Option<&str>,
    config: &ServerConfig,
    template: &CreativeGraphTemplate,
) -> Result<(), McpError> {
    validate_id(&template.project_id, "project_id")?;
    validate_id(&template.template_id, "template_id")?;
    let project = load_project(cwd, config, &template.project_id)?;
    validate_graph_template(template, &project, config)?;
    let path = template_path(&template.project_id, &template.template_id);
    if state_exists(cwd, config, &path)? {
        return Err(McpError::InvalidRequest(
            "creative graph template already exists; save a new template identity/version".into(),
        ));
    }
    let mut index = read_index(cwd, config, &template.project_id)?;
    if index.template_ids.len() >= MAX_TEMPLATES_PER_PROJECT {
        return Err(McpError::InvalidRequest(
            "creative graph template capacity reached".into(),
        ));
    }
    write_json(cwd, config, &path, template, false)?;
    index.template_ids.push(template.template_id.clone());
    index.template_ids.sort();
    index.template_ids.dedup();
    write_json(cwd, config, &index_path(&template.project_id), &index, true)
}

pub fn load_template(
    cwd: Option<&str>,
    config: &ServerConfig,
    project_id: &str,
    template_id: &str,
) -> Result<CreativeGraphTemplate, McpError> {
    validate_id(project_id, "project_id")?;
    validate_id(template_id, "template_id")?;
    let project = load_project(cwd, config, project_id)?;
    let index = read_index(cwd, config, project_id)?;
    if !index.template_ids.iter().any(|value| value == template_id) {
        return Err(McpError::InvalidRequest(
            "unknown creative graph template".into(),
        ));
    }
    let template: CreativeGraphTemplate =
        read_json(cwd, config, &template_path(project_id, template_id))?;
    validate_graph_template(&template, &project, config)?;
    Ok(template)
}

pub fn list_templates(
    cwd: Option<&str>,
    config: &ServerConfig,
    project_id: &str,
) -> Result<Vec<CreativeGraphTemplate>, McpError> {
    let index = read_index(cwd, config, project_id)?;
    index
        .template_ids
        .iter()
        .map(|template_id| load_template(cwd, config, project_id, template_id))
        .collect()
}

fn read_index(
    cwd: Option<&str>,
    config: &ServerConfig,
    project_id: &str,
) -> Result<TemplateIndex, McpError> {
    validate_id(project_id, "project_id")?;
    match read_json::<TemplateIndex>(cwd, config, &index_path(project_id)) {
        Ok(index) => {
            if index.schema_version != CREATIVE_SCHEMA_VERSION
                || index.template_ids.len() > MAX_TEMPLATES_PER_PROJECT
            {
                return Err(McpError::InvalidRequest(
                    "creative graph template index is invalid".into(),
                ));
            }
            for template_id in &index.template_ids {
                validate_id(template_id, "template_id")?;
            }
            Ok(index)
        }
        Err(McpError::InvalidRequest(message))
            if message == "path does not exist or is inaccessible" =>
        {
            Ok(TemplateIndex {
                schema_version: CREATIVE_SCHEMA_VERSION,
                template_ids: Vec::new(),
            })
        }
        Err(error) => Err(error),
    }
}

fn index_path(project_id: &str) -> String {
    format!(".masihawam/creative/projects/{project_id}/templates/index.json")
}

fn template_path(project_id: &str, template_id: &str) -> String {
    format!(".masihawam/creative/projects/{project_id}/templates/{template_id}.json")
}
