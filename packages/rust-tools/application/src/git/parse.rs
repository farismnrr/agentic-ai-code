use super::MAX_GIT_RESULTS;

pub(super) fn status_path(record: &str) -> Option<String> {
    let fields = if record.starts_with("1 ") {
        9
    } else if record.starts_with("2 ") {
        10
    } else if record.starts_with("u ") {
        11
    } else {
        return None;
    };
    record
        .splitn(fields, ' ')
        .nth(fields - 1)
        .map(str::to_owned)
}

pub(super) fn push_bounded(target: &mut Vec<String>, value: String, truncated: &mut bool) {
    if target.len() < MAX_GIT_RESULTS {
        target.push(value)
    } else {
        *truncated = true
    }
}
