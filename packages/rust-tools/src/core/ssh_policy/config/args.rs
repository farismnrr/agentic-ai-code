use super::SshConnectionSpec;
use crate::core::ssh_policy::ValidatedRemoteCommand;
use std::path::Path;

pub fn openssh_args(spec: &SshConnectionSpec, remote: &ValidatedRemoteCommand) -> Vec<String> {
    openssh_args_for_chain(std::slice::from_ref(spec), remote)
}

pub fn openssh_args_for_chain(
    chain: &[SshConnectionSpec],
    remote: &ValidatedRemoteCommand,
) -> Vec<String> {
    openssh_args_for_chain_with_program(chain, remote, Path::new("/usr/bin/ssh"))
}

pub fn openssh_args_for_chain_with_program(
    chain: &[SshConnectionSpec],
    remote: &ValidatedRemoteCommand,
    openssh_program: &Path,
) -> Vec<String> {
    let target = chain.last().expect("SSH chain always contains a target");
    let mut args = common_args(target, !remote.requires_redis_password);
    if chain.len() > 1 {
        let proxy_command = build_proxy_command(chain, chain.len() - 2, target, openssh_program);
        args.extend(["-o".into(), format!("ProxyCommand={proxy_command}")]);
    }
    if let Some(user) = &target.user {
        args.extend(["-l".into(), user.clone()]);
    }
    args.push(target.hostname.clone());
    args.push(remote.rendered.clone());
    args
}

fn common_args(spec: &SshConnectionSpec, stdin_null: bool) -> Vec<String> {
    let mut args = vec![
        "-F".into(),
        "/dev/null".into(),
        "-o".into(),
        "BatchMode=yes".into(),
        "-o".into(),
        "PasswordAuthentication=no".into(),
        "-o".into(),
        "KbdInteractiveAuthentication=no".into(),
        "-o".into(),
        "PreferredAuthentications=publickey".into(),
        "-o".into(),
        "NumberOfPasswordPrompts=0".into(),
        "-o".into(),
        "IdentitiesOnly=yes".into(),
        "-o".into(),
        "IdentityAgent=none".into(),
        "-o".into(),
        "ClearAllForwardings=yes".into(),
        "-o".into(),
        "ForwardAgent=no".into(),
        "-o".into(),
        "ForwardX11=no".into(),
        "-o".into(),
        "PermitLocalCommand=no".into(),
        "-o".into(),
        "ControlMaster=no".into(),
        "-o".into(),
        "ControlPersist=no".into(),
        "-o".into(),
        "RequestTTY=no".into(),
        "-o".into(),
        "EscapeChar=none".into(),
        "-o".into(),
        "StrictHostKeyChecking=yes".into(),
        "-o".into(),
        "UpdateHostKeys=no".into(),
        "-o".into(),
        "ConnectionAttempts=1".into(),
        "-o".into(),
        "ConnectTimeout=10".into(),
        "-o".into(),
        format!("UserKnownHostsFile={}", spec.known_hosts_file.display()),
        "-i".into(),
        spec.identity_file.to_string_lossy().into_owned(),
        "-p".into(),
        spec.port.to_string(),
    ];
    if stdin_null {
        args.extend(["-o".into(), "StdinNull=yes".into()]);
    } else {
        args.extend(["-o".into(), "StdinNull=no".into()]);
    }
    args
}

fn build_proxy_command(
    chain: &[SshConnectionSpec],
    hop_index: usize,
    next: &SshConnectionSpec,
    openssh_program: &Path,
) -> String {
    let hop = &chain[hop_index];
    let mut args = common_args(hop, true);
    if hop_index > 0 {
        let nested = build_proxy_command(chain, hop_index - 1, hop, openssh_program);
        args.extend(["-o".into(), format!("ProxyCommand={nested}")]);
    }
    args.extend(["-W".into(), format!("{}:{}", next.hostname, next.port)]);
    if let Some(user) = &hop.user {
        args.extend(["-l".into(), user.clone()]);
    }
    args.push(hop.hostname.clone());
    render_argv(openssh_program, &args)
}

fn render_argv(program: &Path, args: &[String]) -> String {
    let program = program.to_string_lossy().into_owned();
    std::iter::once(program.as_str())
        .chain(args.iter().map(String::as_str))
        .map(|value| shell_words::quote(value).into_owned())
        .collect::<Vec<_>>()
        .join(" ")
}
