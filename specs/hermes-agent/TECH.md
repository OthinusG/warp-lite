# Hermes Agent Detection and Icon Restoration — Technical Plan

## Implementation

- Add `Icon::HermesLogo` mapped to `bundled/png/hermes.png`.
- Wire `CLIAgent::icon()` to return `Some(Icon::HermesLogo)` instead of `None`.
- Bundle a transparent RGBA Hermes bust PNG and render it as an original-color image in CLI Agent tab/status circles.
- Accept both `hermes` and `hermes-agent` command prefixes with arguments.
- Add `hermes` to the input classifier shell keyword set.
- Maintain the implementation directly in repository source; no patch replay or restoration script is required.

## Verification

- Run focused CLI Agent detection tests.
- Validate native integration tests, formatting and diff hygiene.
