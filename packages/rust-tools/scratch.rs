#[cfg(unix)]
fn check_root() -> Result<(), Box<dyn std::error::Error>> {
    if unsafe { libc::geteuid() } == 0 {
        return Err("refusing to start as UID 0 (root)".into());
    }
    Ok(())
}
#[cfg(not(unix))]
fn check_root() -> Result<(), Box<dyn std::error::Error>> {
    Ok(())
}
