# DeepSeek Harness CLI Agent Detection — Technical Plan

## Implementation

- Add `CLIAgent::DeepSeekHarness` and exhaustive fallbacks.
- Special-case detection after alias resolution; accept only the standalone `dsh-tui` command or `dsh` TUI mode/profile arguments, and reject the `plugin` subcommand.
- Use the existing no-listener, no-plugin, and inline-submit paths; bundle `resources/148330874.png` as the CLI Agent icon.
- Add `dsh` to the input classifier shell keyword set.
- Store the source changes in `.github/patches/deepseek-harness.patch` and apply it from `script/restore-project-explorer.sh` after the Antigravity patch.
- Render the RGB DeepSeek PNG as an original-color image in CLI Agent tab/status circles instead of passing it through the monochrome icon tint path.

## Verification

- Run focused CLI Agent detection tests.
- Run `cargo test -p input_classifier` and `cargo check -p warp --bin warp-oss`.
- Validate patch replay, script syntax, formatting, and diff hygiene.
