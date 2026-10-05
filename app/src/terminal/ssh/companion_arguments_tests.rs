use super::companion_ssh_arguments;
use crate::terminal::shell::ShellType;

#[test]
fn companion_transport_preserves_destination_and_native_authentication_paths() {
    let arguments = companion_ssh_arguments(
        r#"ssh -p 2222 -J jump-user@jump -i 'C:\Keys with spaces\多语言\id' user@host"#,
        ShellType::PowerShell.into(),
    )
    .unwrap();
    assert_eq!(
        arguments,
        [
            "-p",
            "2222",
            "-J",
            "jump-user@jump",
            "-i",
            r"C:\Keys with spaces\多语言\id",
            "user@host"
        ]
    );
    assert_eq!(
        companion_ssh_arguments(r"ssh -i C:\Keys\id user@host", ShellType::PowerShell.into())
            .unwrap(),
        ["-i", r"C:\Keys\id", "user@host"]
    );
    assert_eq!(
        companion_ssh_arguments(
            "command ssh -tt -L 8080:localhost:80 user@host",
            ShellType::Bash.into()
        )
        .unwrap(),
        ["user@host"]
    );
    for command in [
        "ssh host remote-command",
        "ssh -T host",
        "ssh host | another",
        "ssh host > output",
        "ssh -i $KEY_PATH host",
        "ssh host; another",
        "CUSTOM_SSH_ENV=value ssh host",
        "ssh -o RemoteCommand=another host remote-command",
    ] {
        assert!(
            companion_ssh_arguments(command, ShellType::Bash.into()).is_none(),
            "Non-literal or noninteractive SSH must stay native"
        );
    }
}
