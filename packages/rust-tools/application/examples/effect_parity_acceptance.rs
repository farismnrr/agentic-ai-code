use relay_application::hooks::effect_classes;
use serde_json::json;

fn main() {
    let cases = [
        ("directory_list", false, false),
        ("file_write", false, false),
        ("git_diff", false, false),
        ("terminal_exec", false, false),
        ("http_fetch", false, false),
        ("external_tool", true, false),
        ("external_search", false, true),
    ];
    let output: Vec<_> = cases.into_iter().map(|(tool, destructive, open_world)| {
        json!({"tool": tool, "destructive": destructive, "open_world": open_world, "effects": effect_classes(tool, destructive, open_world)})
    }).collect();
    println!("{}", serde_json::to_string(&output).unwrap());
}
