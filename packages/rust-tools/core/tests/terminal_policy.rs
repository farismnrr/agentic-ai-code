use relay_core::terminal_policy::validate_executable;

#[test]
fn privilege_escalation_commands_remain_forbidden() {
    for command in ["sudo", "su", "doas", "pkexec", "runas"] {
        assert!(validate_executable(command, true).is_err(), "{command}");
    }
}

#[test]
fn docker_requires_explicit_opt_in() {
    assert!(validate_executable("docker", false).is_err());
    assert!(validate_executable("docker", true).is_ok());
}

#[test]
fn executable_paths_remain_forbidden() {
    assert!(validate_executable("/usr/bin/docker", true).is_err());
    assert!(validate_executable("../docker", true).is_err());
}
