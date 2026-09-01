use ai_tools::application::workspace::file_search;
use ai_tools::core::config::ServerConfig;
use serde_json::json;

fn main() {
    let config = ServerConfig::default();

    let oversized_pattern = "a".repeat(4_097);
    let error = file_search(&json!({"pattern": oversized_pattern}), &config)
        .expect_err("native file_search must enforce the pattern byte cap");
    assert!(error.to_string().contains("relative glob"));

    let oversized_cwd = "x".repeat(4_097);
    let error = file_search(&json!({"pattern": "*.rs", "cwd": oversized_cwd}), &config)
        .expect_err("native file_search must enforce the cwd byte cap");
    assert!(error.to_string().contains("cwd exceeds maximum"));

    let too_many_segments = std::iter::repeat_n("x", 129).collect::<Vec<_>>().join("/");
    let error = file_search(&json!({"pattern": too_many_segments}), &config)
        .expect_err("native file_search must enforce the pattern segment cap");
    assert!(error.to_string().contains("relative glob"));

    println!("file_search native limits: PASS");
}
