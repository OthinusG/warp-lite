//! Installed CLI option contracts separate interactive configuration from initial work.
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Serialize, Deserialize)]
pub enum Arity {
    Blocked,
    BlockedValue,
    BlockedOptionalValue,
    BlockedValues,
    Flag,
    Value,
    OptionalValue,
    Values,
}

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct LaunchOptions(pub BTreeMap<String, Arity>);

impl LaunchOptions {
    /// Unknown help layouts stay conservative; they cannot establish an idle lease.
    pub fn from_help(program: &str, help: &str) -> Self {
        let mut options = BTreeMap::new();
        for line in help
            .lines()
            .map(str::trim_start)
            .filter(|line| line.starts_with('-'))
        {
            let syntax = line.split("  ").next().unwrap_or(line);
            let names: Vec<_> = syntax
                .split_whitespace()
                .map(|word| word.trim_end_matches(','))
                .filter(|name| {
                    name.starts_with('-')
                        && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
                })
                .collect();
            let blocked = names.iter().any(|name| {
                matches!(
                    *name,
                    "--print"
                        | "--prompt"
                        | "--prompt-interactive"
                        | "--execute"
                        | "--headless"
                        | "--background"
                        | "--bg"
                        | "--remote"
                        | "--remote-session"
                        | "--remote-control"
                        | "--teleport"
                        | "--continue"
                        | "--resume"
                        | "--conversation"
                        | "--fork-session"
                        | "--from-pr"
                        | "--help"
                        | "--version"
                        | "--list-models"
                        | "--list-sessions"
                        | "--delete-session"
                        | "--dump-config"
                        | "--dump-default-config"
                        | "--image"
                        | "--attachment"
                        | "--input-format"
                        | "--output-format"
                )
            });
            let arity = if syntax.contains("...") {
                Arity::Values
            } else if syntax.contains('<') {
                Arity::Value
            } else if syntax.contains('[') {
                Arity::OptionalValue
            } else {
                Arity::Flag
            };
            let arity = if blocked {
                match arity {
                    Arity::Value => Arity::BlockedValue,
                    Arity::OptionalValue => Arity::BlockedOptionalValue,
                    Arity::Values => Arity::BlockedValues,
                    _ => Arity::Blocked,
                }
            } else {
                arity
            };
            for name in names {
                options.insert(name.to_owned(), arity);
            }
        }
        if program == "codex" {
            // Codex exposes this permission-mode alias even when long help omits it.
            options.insert("--yolo".into(), Arity::Flag);
        }
        if program == "agy" {
            // This client's Go help omits value placeholders and prints aliases separately.
            for name in [
                "--model",
                "--agent",
                "--effort",
                "--mode",
                "--log-file",
                "--add-dir",
                "--json-schema",
                "--print-timeout",
            ] {
                if options.contains_key(name) {
                    options.insert(name.into(), Arity::Value);
                }
            }
            for name in ["-c", "-i", "-p"] {
                if options.contains_key(name) {
                    options.insert(name.into(), Arity::Blocked);
                }
            }
        }
        Self(options)
    }

    /// This never affects agent recognition: uncertain or task-bearing launches await lifecycle readiness.
    pub fn is_empty_interactive(&self, args: &[String]) -> bool {
        let mut args = args.iter().peekable();
        while let Some(arg) = args.next() {
            if arg == "--" {
                return args.next().is_none();
            }
            let (mut name, mut inline) = arg
                .split_once('=')
                .map_or((arg.as_str(), None), |(name, value)| (name, Some(value)));
            // Accept attached short-option values without guessing unknown option clusters.
            if !self.0.contains_key(name)
                && name.starts_with('-')
                && !name.starts_with("--")
                && name.is_ascii()
                && name.len() > 2
            {
                if self
                    .0
                    .get(&name[..2])
                    .is_some_and(|arity| matches!(arity, Arity::Value | Arity::Values))
                {
                    inline = Some(&arg[2..]);
                    name = &arg[..2];
                }
            }
            let Some(arity) = self.0.get(name) else {
                return false;
            };
            match (arity, inline) {
                (
                    Arity::Blocked
                    | Arity::BlockedValue
                    | Arity::BlockedOptionalValue
                    | Arity::BlockedValues,
                    _,
                ) => return false,
                (Arity::Flag, Some(_)) => return false,
                (_, Some("")) => return false,
                (Arity::Flag, None) | (_, Some(_)) => {}
                (Arity::Value | Arity::Values, None) => {
                    if args.next().is_none_or(|value| value.starts_with('-')) {
                        return false;
                    }
                    if matches!(arity, Arity::Values) {
                        while args.peek().is_some_and(|value| !value.starts_with('-')) {
                            args.next();
                        }
                    }
                }
                (Arity::OptionalValue, None) => {
                    if args.peek().is_some_and(|value| !value.starts_with('-')) {
                        args.next();
                    }
                }
            }
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn launch_options_preserve_configuration_and_reject_initial_work() {
        let options = LaunchOptions::from_help("codex", "  --no-daemon  Use this process\n  -m, --model <MODEL>  Model\n  -p, --profile <PROFILE>  Profile\n  --add-dir <DIR>  Directory\n  --worktree [NAME]  Worktree\n  --search  Search");
        for args in [
            "",
            "--yolo",
            "--no-daemon --yolo -m model -p profile",
            "-mmodel -pprofile",
            "--model=model --add-dir project",
            "--worktree name --search",
        ] {
            let args = args
                .split_whitespace()
                .map(str::to_owned)
                .collect::<Vec<_>>();
            assert!(options.is_empty_interactive(&args));
        }
        for args in [
            "--yolo task",
            "exec task",
            "--model",
            "--model=",
            "--model --yolo",
            "--unknown",
            "--yolo=true",
            "--no-daemon -- task",
        ] {
            let args = args
                .split_whitespace()
                .map(str::to_owned)
                .collect::<Vec<_>>();
            assert!(!options.is_empty_interactive(&args), "{args:?}");
        }
        let options = LaunchOptions::from_help("qoder", "  -p, --print  Print\n  --model <MODEL>  Model\n  --dangerously-skip-permissions  Permissions");
        assert!(options.is_empty_interactive(&[
            "--model".into(),
            "model".into(),
            "--dangerously-skip-permissions".into()
        ]));
        assert!(!options.is_empty_interactive(&["-p".into()]));
        assert!(!options.is_empty_interactive(&["--print".into()]));
        let options = LaunchOptions::from_help("agy", "  --model  Model\n  -c  Short alias for --continue\n  --continue  Continue\n  --conversation  Resume by ID");
        assert!(options.is_empty_interactive(&["--model".into(), "model".into()]));
        assert!(!options.is_empty_interactive(&["-c".into()]));
        assert!(!options.is_empty_interactive(&["--conversation".into(), "id".into()]));
    }
}
