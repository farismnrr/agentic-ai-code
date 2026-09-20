use ai_tools::core::config::ServerConfig;
use clap::{Args as ClapArgs, Parser, ValueEnum};
use serde_json::Value;
use std::path::PathBuf;

#[derive(ClapArgs, Debug)]
pub struct Args {
    /// Heavy Creative/Blender operation to execute in the foreground.
    #[arg(long, value_enum)]
    tool: OperatorTool,
    /// JSON file containing the same internal argument object used by the reviewed operation.
    #[arg(long)]
    input: PathBuf,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum OperatorTool {
    CreativeJob,
    CreativeGraph,
    BlenderExecutePython,
    BlenderAnimationPreview,
    BlenderRender,
    BlenderAssetImport,
    BlenderAssetExport,
    BlenderCheckpointCreate,
    BlenderCheckpointRestore,
}

impl OperatorTool {
    fn tool_name(self) -> &'static str {
        match self {
            Self::CreativeJob => "creative_job",
            Self::CreativeGraph => "creative_graph",
            Self::BlenderExecutePython => "blender_execute_python",
            Self::BlenderAnimationPreview => "blender_animation_preview",
            Self::BlenderRender => "blender_render",
            Self::BlenderAssetImport => "blender_asset_import",
            Self::BlenderAssetExport => "blender_asset_export",
            Self::BlenderCheckpointCreate => "blender_checkpoint_create",
            Self::BlenderCheckpointRestore => "blender_checkpoint_restore",
        }
    }

    fn validate_action(self, arguments: &Value) -> Result<(), Box<dyn std::error::Error>> {
        let action = arguments.get("action").and_then(Value::as_str);
        match self {
            Self::CreativeJob if action == Some("wait") => Ok(()),
            Self::CreativeGraph
                if matches!(
                    action,
                    Some(
                        "execute"
                            | "partial_rerun"
                            | "rerun_selected"
                            | "rerun_subgraph"
                            | "rerun_all_dirty"
                    )
                ) =>
            {
                Ok(())
            }
            Self::CreativeJob => Err(
                "operator creative-job execution only accepts action=wait; submit/get/list/cancel remain MCP control-plane operations"
                    .into(),
            ),
            Self::CreativeGraph => Err(
                "operator creative-graph execution requires execute or a rerun action".into(),
            ),
            _ if action.is_some() => Err(
                "Blender operator requests do not accept a generic action field".into(),
            ),
            _ => Ok(()),
        }
    }
}

pub async fn run(args: Args) -> Result<(), Box<dyn std::error::Error>> {
    let raw = std::fs::read_to_string(&args.input)?;
    let arguments: Value = serde_json::from_str(&raw)?;
    if !arguments.is_object() {
        return Err("operator request must be a JSON object".into());
    }
    args.tool.validate_action(&arguments)?;

    // Reuse the same reviewed RELAY_* environment contract as the running relay.
    // The operator CLI is foreground-only and intentionally has no timeout wrapper.
    let relay_cli = ai_tools::core::config::Cli::try_parse_from(["ai-tools-creative-operator"])?;
    let config = ServerConfig::from(&relay_cli);
    if !config.enable_creative {
        return Err(
            "Creative production is disabled in the operator environment; set RELAY_ENABLE_CREATIVE=true using the same reviewed relay configuration"
                .into(),
        );
    }
    config.ensure_workspaces_initialized()?;
    let owner = config.oauth_owner_subject.as_deref().unwrap_or("local");

    let result = if args.tool.tool_name().starts_with("blender_") {
        let operator_config = ai_tools::application::blender::unbounded_operator_config(&config);
        ai_tools::application::blender::dispatch_tool(
            args.tool.tool_name(),
            &arguments,
            &operator_config,
            owner,
        )
        .await?
    } else {
        ai_tools::application::creative::dispatch_tool(
            args.tool.tool_name(),
            &arguments,
            &config,
            owner,
        )
        .await?
    }
    .ok_or("operator tool is not implemented")?;

    println!("{}", serde_json::to_string_pretty(&result)?);
    if result.is_error {
        return Err("operator execution returned a tool error".into());
    }
    Ok(())
}
