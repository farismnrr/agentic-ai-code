use relay_application::execution::{JobSnapshot, JobState};
use serde_json::json;

fn snapshot(state: JobState) -> JobSnapshot {
    JobSnapshot {
        job_id: "task-progress-test".into(),
        state,
        created_at: 1_000,
        last_updated_at: 2_000,
        started_at: Some(1_100),
        finished_at: matches!(state, JobState::TimedOut).then_some(2_000),
        execution_duration_ms: Some(900),
        stdout: "step 1\nBearer should-not-leak\n".into(),
        stderr: "partial stderr\n".into(),
        omitted_bytes: 7,
        exit_code: None,
        result: None,
    }
}

#[test]
fn task_responses_include_bounded_progress_output_for_running_jobs() {
    let task = snapshot(JobState::Running);

    assert_eq!(
        task.create_task_json()["output"],
        json!({
            "stdout": "step 1\nBearer [REDACTED]\n",
            "stderr": "partial stderr\n",
            "omittedBytes": 7,
            "exitCode": null
        })
    );
    assert_eq!(
        task.task_json(60_000)["output"],
        task.create_task_json()["output"]
    );
}

#[test]
fn timed_out_task_retains_last_progress_for_resume() {
    let task = snapshot(JobState::TimedOut);
    let response = task.task_json(60_000);

    assert_eq!(response["status"], "completed");
    assert_eq!(response["executionStatus"], "timed_out");
    assert_eq!(response["output"]["stdout"], "step 1\nBearer [REDACTED]\n");
    assert_eq!(response["output"]["stderr"], "partial stderr\n");
}
