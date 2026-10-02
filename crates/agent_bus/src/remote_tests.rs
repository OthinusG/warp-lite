use super::*;
use std::{ffi::OsStr, time::Duration};

#[test]
fn ssh_arguments_preserve_paths_and_reject_option_or_shell_injection() {
    let directory = tempfile::tempdir().unwrap();
    let executable = directory.path().join("OpenSSH with spaces").join("ssh.exe");
    let command = build_ssh_command(&executable, "workstation-test").unwrap();
    assert_eq!(command.as_std().get_program(), executable.as_os_str());
    let args: Vec<_> = command.as_std().get_args().collect();
    assert_eq!(args[0], OsStr::new("-T"));
    assert_eq!(
        &args[args.len() - 2..],
        &[OsStr::new("workstation-test"), OsStr::new(GATEWAY_COMMAND)]
    );
    for option in SSH_OPTIONS {
        assert!(args
            .windows(2)
            .any(|pair| pair == [OsStr::new("-o"), OsStr::new(option)]));
    }
    for alias in [
        "",
        "-oProxyCommand=anything",
        "host;command",
        "host name",
        "host\ncommand",
        "host$(command)",
        "host`command`",
        "user@host",
        "host/relative",
        "主机",
    ] {
        let error = build_ssh_command(&executable, alias).err().unwrap();
        assert_eq!(
            error.downcast_ref::<crate::DomainError>().unwrap().code,
            "invalid_input"
        );
        assert!(!error.to_string().contains(alias) || alias.is_empty());
    }
    assert!(build_ssh_command(&executable, &"a".repeat(254)).is_err());
}

#[tokio::test]
async fn system_ssh_accepts_strict_background_options_without_connecting() {
    let executable = match ssh_executable() {
        Ok(executable) => executable,
        Err(error) => {
            assert_eq!(
                error.downcast_ref::<crate::DomainError>().unwrap().code,
                "feature_unavailable"
            );
            eprintln!("System OpenSSH unavailable; real executable acceptance remains unverified");
            return;
        }
    };
    let empty_config = tempfile::NamedTempFile::new().unwrap();
    let channel = build_ssh_command(&executable, "collaboration-test.invalid").unwrap();
    let mut probe = Command::new(&executable);
    // -G parses options without networking; the empty config avoids user hooks/credentials.
    probe
        .arg("-G")
        .arg("-F")
        .arg(empty_config.path())
        .args(channel.as_std().get_args())
        .stdin(Stdio::null())
        .kill_on_drop(true);
    let output = tokio::time::timeout(Duration::from_secs(10), probe.output())
        .await
        .expect("SSH configuration probe timed out")
        .expect("SSH probe failed to start");
    assert!(
        output.status.success(),
        "System SSH rejected collaboration options"
    );
    let configuration = String::from_utf8(output.stdout).unwrap();
    for setting in [
        "batchmode yes",
        "stricthostkeychecking true",
        "forwardagent no",
        "forwardx11 no",
        "clearallforwardings yes",
        "permitlocalcommand no",
        "requesttty false",
        "stdinnull no",
        "connecttimeout 10",
        "serveraliveinterval 10",
        "serveralivecountmax 3",
        "controlmaster false",
        "controlpersist no",
    ] {
        assert!(
            configuration.lines().any(|line| line == setting),
            "Missing safe SSH setting: {setting}"
        );
    }
}
