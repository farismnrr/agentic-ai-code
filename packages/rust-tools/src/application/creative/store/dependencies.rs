use super::support::new_id;
use crate::application::creative::contracts::{
    CreativeProject, QaFinding, QaSeverity, MAX_QA_FINDINGS,
};
use crate::core::error::McpError;
use std::collections::HashSet;

pub(super) fn mark_style_dependents_for_review(
    project: &mut CreativeProject,
    style_element_id: &str,
    style_revision_id: &str,
    created_at_ms: u128,
) -> Result<usize, McpError> {
    let mut subjects = HashSet::new();
    for asset in &project.assets {
        if asset
            .dependency_element_ids
            .iter()
            .any(|dependency_id| dependency_id == style_element_id)
        {
            subjects.insert((asset.asset_id.clone(), "asset"));
        }
    }
    for scene in &project.scenes {
        if scene.style_element_id.as_deref() == Some(style_element_id) {
            for shot in &scene.shots {
                subjects.insert((shot.shot_id.clone(), "shot"));
            }
        }
    }

    if project.qa_findings.len().saturating_add(subjects.len()) > MAX_QA_FINDINGS {
        return Err(McpError::InvalidRequest(
            "style dependency review findings would exceed project capacity".into(),
        ));
    }

    let mut subjects = subjects.into_iter().collect::<Vec<_>>();
    subjects.sort_by(|left, right| left.0.cmp(&right.0));
    for (subject_id, subject_kind) in &subjects {
        project.qa_findings.push(QaFinding {
            finding_id: new_id("qa"),
            domain: "style_dependency".into(),
            severity: QaSeverity::SoftFinding,
            subject_id: subject_id.clone(),
            message: format!(
                "{subject_kind} depends on Style Element {style_element_id}; review after promoted revision {style_revision_id}"
            ),
            source_revision_id: Some(style_revision_id.to_owned()),
            asset_id: (subject_kind == &"asset").then(|| subject_id.clone()),
            evaluator_binding_id: None,
            evidence: serde_json::json!({
                "style_element_id": style_element_id,
                "style_revision_id": style_revision_id,
                "subject_kind": subject_kind
            }),
            created_at_ms,
        });
    }
    Ok(subjects.len())
}
