//! Native per-invocation Codex adaptation; no vendor files, aliases or PATH writes.
use super::{setup, AgentCommunication};
use crate::terminal::{
    model::session::command_executor::shell_quote_arg, shell::ShellType, view::ExecuteCommandEvent,
    TerminalView,
};
use crate::view_components::ToastFlavor;
use warpui::{SingletonEntity, ViewContext};

fn session_prefix_for_shell(prefix: &[String], shell: ShellType) -> String {
    prefix.iter().map(|arg| {
        if shell == ShellType::PowerShell && arg.contains('"') {
            // This compact override has metacharacters/whitespace only in basic strings.
            // TOML escapes preserve their values through legacy quoting and .cmd parsing.
            let encoded = arg.chars().map(|c| {
                if " &^%!|<>()`$".contains(c) { format!("\\u{:04x}", u32::from(c)) }
                else { c.to_string() }
            }).collect::<String>();
            let literal = shell_quote_arg(&encoded, shell);
            // Windows PowerShell and pwsh Legacy mode drop native embedded quotes.
            let legacy = shell_quote_arg(&encoded.replace('"', "\\\""), shell);
            format!("$(if ($PSVersionTable.PSVersion.Major -lt 7 -or ($PSVersionTable.PSVersion.Major -eq 7 -and $PSVersionTable.PSVersion.Minor -lt 3) -or $PSNativeCommandArgumentPassing -eq 'Legacy' -or ($PSNativeCommandArgumentPassing -eq 'Windows' -and (Get-Command codex).Source -match '\\.(cmd|bat)$')) {{ {legacy} }} else {{ {literal} }})")
        } else {
            shell_quote_arg(arg, shell)
        }
    }).collect::<Vec<_>>().join(" ")
}

fn adapted_command(
    command: &str,
    prefix: &[String],
    shell: ShellType,
    options: &warp_agent_bus::launch::LaunchOptions,
) -> String {
    let words = shlex::split(command).unwrap();
    let prefix = if warp_agent_bus::session::codex_needs_no_daemon(&words[1..], options) {
        prefix
    } else {
        &prefix[1..]
    };
    let tail = command.trim_start().strip_prefix("codex").unwrap();
    format!("codex {}{tail}", session_prefix_for_shell(prefix, shell))
}

impl TerminalView {
    pub(crate) fn codex_original_command<'a>(&'a self, command: &'a str) -> &'a str {
        self.codex_mcp_launch
            .as_ref()
            .filter(|(_, adapted)| adapted == command)
            .map(|(original, _)| original.as_str())
            .unwrap_or(command)
    }

    pub(crate) fn adapt_codex_mcp_launch(
        &mut self,
        event: &ExecuteCommandEvent,
        ctx: &mut ViewContext<Self>,
    ) -> bool {
        self.codex_mcp_launch_generation = self.codex_mcp_launch_generation.wrapping_add(1);
        let generation = self.codex_mcp_launch_generation;
        let trimmed = event.command.trim_start();
        let Some(_) = trimmed
            .strip_prefix("codex")
            .filter(|tail| tail.is_empty() || tail.starts_with(char::is_whitespace))
        else {
            return false;
        };
        let Some(words) = shlex::split(trimmed) else {
            return false;
        };
        if words.first().map(String::as_str) != Some("codex") {
            return false;
        }
        #[cfg(windows)]
        if self.adapt_wsl_codex_launch(event, &words, ctx) {
            return true;
        }
        if !self.session_is_local(event.session_id, ctx) {
            return false;
        }
        let settings = AgentCommunication::as_ref(ctx);
        if !settings.preferences.enabled || !settings.preferences.programs().contains("codex") {
            return false;
        }
        let Some(installed) = settings
            .preferences
            .selected
            .values()
            .find(|entry| entry.active && entry.program == "codex")
        else {
            return false;
        };
        // Administrative/protocol commands stay native and do not require a communication bridge.
        if !warp_agent_bus::session::codex_accepts_peer_prompt(
            &words[1..],
            &installed.launch_options,
        ) {
            return false;
        }
        let Some(shell_type) = self.active_session_shell_type(ctx) else {
            return false;
        };
        let Some(directory) = self.active_session_path_if_local(ctx) else {
            return false;
        };
        let Some(session) = self.sessions_model().as_ref(ctx).get(event.session_id) else {
            return false;
        };
        // A Windows host probe cannot establish the Linux CLI/IPC contract of a WSL guest.
        if session.is_wsl()
            || session.alias_value("codex").is_some()
            || session.abbreviation_value("codex").is_some()
            || session.function_names().any(|name| name == "codex")
        {
            return false;
        }
        let search_paths = self
            .sessions_model()
            .as_ref(ctx)
            .env_var_for_session(event.session_id, "PATH")
            .or_else(|| session.path().as_deref())
            .map(|path| std::env::split_paths(std::ffi::OsStr::new(path)).collect::<Vec<_>>())
            .unwrap_or_default();
        let event = event.clone();
        ctx.spawn(
            async move { tokio::task::spawn_blocking(move || setup::companion().and_then(|bridge| setup::codex_session_options(&search_paths, &bridge))).await },
            move |view, result, ctx| {
                if view.codex_mcp_launch_generation != generation { return; }
                if view.active_block_session_id() != Some(event.session_id)
                    || view.active_session_path_if_local(ctx).as_ref() != Some(&directory)
                    || view.is_long_running()
                {
                    view.show_persistent_toast("Codex launch canceled because the terminal changed.".into(), ToastFlavor::Default, ctx);
                    return;
                }
                let settings = AgentCommunication::as_ref(ctx);
                if !settings.preferences.enabled || !settings.preferences.programs().contains("codex") {
                    view.execute_input_command(&event, ctx);
                    return;
                }
                let (prefix, options) = match result {
                    Ok(Ok(value)) => value,
                    _ => {
                        view.show_persistent_toast("Codex started without session MCP. Check native --no-daemon support and the bundled bridge.".into(), ToastFlavor::Default, ctx);
                        view.execute_input_command(&event, ctx);
                        return;
                    }
                };
                if !warp_agent_bus::session::codex_accepts_peer_prompt(&words[1..], &options) {
                    view.execute_input_command(&event, ctx);
                    return;
                }
                AgentCommunication::handle(ctx).update(ctx, |settings, _| {
                    for entry in settings.preferences.selected.values_mut().filter(|entry| entry.program == "codex") {
                        entry.launch_options = options.clone();
                    }
                });
                // Keep the original tail verbatim: reserializing it would change shell expansion.
                let command = adapted_command(&event.command, &prefix, shell_type, &options);
                view.codex_mcp_launch = Some((event.command.trim().to_owned(), command.clone()));
                let mut adapted = event;
                adapted.command = command;
                view.execute_input_command(&adapted, ctx);
            },
        );
        true
    }

    #[cfg(windows)]
    fn adapt_wsl_codex_launch(
        &mut self,
        event: &ExecuteCommandEvent,
        words: &[String],
        ctx: &mut ViewContext<Self>,
    ) -> bool {
        let Some(session) = self.sessions_model().as_ref(ctx).get(event.session_id) else {
            return false;
        };
        let Some(target) = super::wsl_settings::Target::from_session(&session) else {
            return false;
        };
        if self.active_block_session_id() != Some(event.session_id)
            || session.alias_value("codex").is_some()
            || session.abbreviation_value("codex").is_some()
            || session.function_names().any(|name| name == "codex")
        {
            return false;
        }
        let settings = &AgentCommunication::as_ref(ctx).wsl;
        if !settings.preferences.enabled || !settings.targets.contains(&target) {
            return false;
        }
        let Some(shell) = self.active_session_shell_type(ctx) else {
            return false;
        };
        let generation = self.codex_mcp_launch_generation;
        let directory = self.model.lock().active_block_metadata()
            .current_working_directory().map(str::to_owned);
        let event = event.clone();
        let words = words.to_vec();
        let selected = target.clone();
        // Read the launching account, independently of which account is selected in settings.
        ctx.spawn(async move {
            super::wsl_settings::guest_setup(&target, warp_agent_bus::wsl_setup::Request {
                action: warp_agent_bus::wsl_setup::Action::Rescan,
                commands: vec![("codex".into(), "codex".into())],
            }).await
        }, move |view, result, ctx| {
            if view.codex_mcp_launch_generation != generation { return; }
            let current_directory = view.model.lock().active_block_metadata()
                .current_working_directory().map(str::to_owned);
            if view.active_block_session_id() != Some(event.session_id)
                || current_directory != directory
                || view.is_long_running()
                || view.sessions_model().as_ref(ctx).get(event.session_id)
                    .and_then(|session| super::wsl_settings::Target::from_session(&session))
                    .as_ref() != Some(&selected)
            {
                view.show_persistent_toast("Codex launch canceled because the terminal changed.".into(), ToastFlavor::Default, ctx);
                return;
            }
            let settings = &AgentCommunication::as_ref(ctx).wsl;
            if !settings.preferences.enabled || !settings.targets.contains(&selected) {
                view.execute_input_command(&event, ctx);
                return;
            }
            let response = match result {
                Ok(response) => response,
                Err(_) => {
                    view.show_persistent_toast("Could not verify WSL Codex MCP. Check communication settings for this account.".into(), ToastFlavor::Default, ctx);
                    view.execute_input_command(&event, ctx);
                    return;
                }
            };
            let options = response.available.iter()
                .find(|entry| entry.program == "codex")
                .and_then(|entry| entry.installed.as_ref())
                .map(|entry| &entry.launch_options);
            let Some(options) = options.filter(|options| {
                response.preferences.programs().contains("codex")
                    && options.0.contains_key("--no-daemon")
                    && warp_agent_bus::session::codex_accepts_peer_prompt(&words[1..], options)
            }) else {
                view.execute_input_command(&event, ctx);
                return;
            };
            // Guest vendor configuration owns MCP; only isolate this native invocation.
            let command = adapted_command(&event.command, &["--no-daemon".into()], shell, options);
            view.codex_mcp_launch = Some((event.command.trim().to_owned(), command.clone()));
            let mut adapted = event;
            adapted.command = command;
            view.execute_input_command(&adapted, ctx);
        });
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[cfg(windows)]
    #[test]
    fn guest_codex_adaptation_only_adds_native_foreground_flag() {
        let options = warp_agent_bus::launch::LaunchOptions::from_help(
            "codex",
            "  --no-daemon  Embedded\n  -m, --model <MODEL>  Model",
        );
        for shell in [ShellType::Bash, ShellType::Zsh, ShellType::Fish] {
            for command in [
                "codex",
                "codex resume --last",
                "codex --no-daemon --yolo",
                "codex --model \"$MODEL\" 'a prompt with spaces'",
                "codex --model=--no-daemon",
            ] {
                let adapted = adapted_command(command, &["--no-daemon".into()], shell, &options);
                let original = shlex::split(command).unwrap();
                let arguments = shlex::split(&adapted).unwrap();
                let inserted = usize::from(warp_agent_bus::session::codex_needs_no_daemon(
                    &original[1..],
                    &options,
                ));
                assert_eq!(&arguments[1 + inserted..], &original[1..]);
                if inserted == 1 {
                    assert_eq!(arguments[1], "--no-daemon");
                }
                assert!(adapted.ends_with(command.strip_prefix("codex").unwrap()));
            }
        }
    }

    #[test]
    fn original_codex_arguments_and_expansion_are_preserved() {
        let prefix = setup::codex_session_prefix(Path::new("/app with spaces/warpai-agent"));
        let options = warp_agent_bus::launch::LaunchOptions::from_help(
            "codex",
            "  --no-daemon  Embedded\n  -m, --model <MODEL>  Model",
        );
        for command in [
            "codex",
            "codex --no-daemon --yolo",
            "codex --model=--no-daemon",
            "codex --yolo",
            "codex resume --last",
            "codex fork session-id",
            "codex --model \"$MODEL\" 'a prompt with spaces'",
            "codex --cd '../other project'",
        ] {
            let adapted = adapted_command(command, &prefix, ShellType::Zsh, &options);
            let arguments = shlex::split(&adapted).unwrap();
            let original = shlex::split(command).unwrap();
            let expected =
                if warp_agent_bus::session::codex_needs_no_daemon(&original[1..], &options) {
                    &prefix[..]
                } else {
                    &prefix[1..]
                };
            assert_eq!(&arguments[1..1 + expected.len()], expected);
            assert_eq!(&arguments[1 + expected.len()..], &original[1..]);
            assert!(adapted.ends_with(command.strip_prefix("codex").unwrap()));
            assert!(adapted.starts_with("codex "));
        }
    }

    #[cfg(windows)]
    #[test]
    fn powershell_preserves_native_mcp_toml_arguments() {
        use std::process::Command;
        let python = Command::new("python")
            .args(["-c", "import sys; print(sys.executable)"])
            .output()
            .unwrap();
        assert!(python.status.success());
        let python = String::from_utf8(python.stdout).unwrap();
        let python = python.trim();
        let directory = tempfile::tempdir().unwrap();
        std::fs::write(
            directory.path().join("codex.cmd"),
            format!("@\"{python}\" -c \"import json,sys; print(json.dumps(sys.argv[1:]))\" %*\r\n"),
        )
        .unwrap();
        let paths = std::env::join_paths(
            std::iter::once(directory.path().to_owned())
                .chain(std::env::split_paths(&std::env::var_os("PATH").unwrap())),
        )
        .unwrap();
        for wrapper in ["cmd", "ps1"] {
            if wrapper == "ps1" {
                std::fs::remove_file(directory.path().join("codex.cmd")).unwrap();
                std::fs::write(
                    directory.path().join("codex.ps1"),
                    format!(
                        "& {} -c 'import json,sys; print(json.dumps(sys.argv[1:]))' @args",
                        shell_quote_arg(python, ShellType::PowerShell),
                    ),
                )
                .unwrap();
            }
            for path in [
                r"C:\app\warpai-agent.exe",
                r"C:\app with spaces\O'Neil\warpai-agent.exe",
                r"C:\app & tools\warpai-agent.exe",
                r"C:\app; (tools) ^ unicode\warpai-agent.exe",
                r"C:\app %WARPAI_DIAGNOSTIC% !tools! 中文\warpai-agent.exe",
            ] {
                let prefix = setup::codex_session_prefix(Path::new(path));
                for shell in ["powershell", "pwsh"] {
                    for mode in ["Legacy", "Windows", "Standard"] {
                        let script = format!(
                            "$PSNativeCommandArgumentPassing = '{mode}'; codex {}",
                            session_prefix_for_shell(&prefix, ShellType::PowerShell)
                        );
                        let output = Command::new(shell)
                            .args(["-NoProfile", "-NonInteractive", "-Command", &script])
                            .env("PATH", &paths)
                            .env("WARPAI_DIAGNOSTIC", "unexpected")
                            .output()
                            .unwrap();
                        assert!(
                            output.status.success(),
                            "{wrapper} {shell} {mode}: {}",
                            String::from_utf8_lossy(&output.stderr)
                        );
                        let actual = serde_json::from_slice::<Vec<String>>(&output.stdout).unwrap();
                        assert_eq!(
                            actual.len(),
                            prefix.len(),
                            "{wrapper} {shell} {mode} {path}"
                        );
                        assert_eq!(&actual[..2], &prefix[..2]);
                        assert_eq!(
                            toml::from_str::<toml::Value>(&actual[2]).unwrap(),
                            toml::from_str::<toml::Value>(&prefix[2]).unwrap(),
                            "{wrapper} {shell} {mode} {path}"
                        );
                    }
                }
            }
        }
    }
}
