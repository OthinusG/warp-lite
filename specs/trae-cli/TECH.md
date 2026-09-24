# Trae CLI Agent Detection — Technical Plan

## Implementation

- Add `CLIAgent::Trae` and exhaustive fallbacks.
- Accept `trae`, `traecn`, `trae-cli`, and `traecn-cli` command prefixes with arguments.
- Use the existing no-listener, no-plugin, and inline-submit paths.
- Add `trae` and `traecn` to the input classifier shell keyword set.
- Store the source changes in `.github/patches/trae-cli.patch` and apply it from `script/restore-project-explorer.sh` after the Qoder CLI patch.
- Bundle a transparent RGBA Trae PNG and render it as an original-color image in CLI Agent tab/status circles instead of passing it through the monochrome icon tint path.

## Verification

- Run focused CLI Agent detection tests.
- Validate patch replay, script syntax, formatting, and diff hygiene.
