# Qoder CLI Agent Detection — Technical Plan

## Implementation

- Add `CLIAgent::Qoder` and exhaustive fallbacks.
- Accept `qoder`, `qodercli`, `qoder-cli`, and domestic `qodercn` command prefixes with arguments.
- Use the existing no-listener, no-plugin, and inline-submit paths.
- Add `qoder` and `qodercn` to the input classifier shell keyword set.
- Maintain the implementation directly in repository source; no patch replay or restoration script is required.
- Bundle a transparent RGBA Qoder PNG and render it as an original-color image in CLI Agent tab/status circles instead of passing it through the monochrome icon tint path.

## Verification

- Run focused CLI Agent detection tests.
- Validate native integration tests, formatting and diff hygiene.
