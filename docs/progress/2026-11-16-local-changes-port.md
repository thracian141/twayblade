# Port local changes onto the current release base

Based on `upstream/main` at `14b3eaa` (0.1.1-rc.1). The original checkout and
its staged/unstaged work are preserved; implementation is on
`feature/port-local-changes` in a separate worktree.

## Behavior

- Ported text/voice channel creation in spaces, forced voice-room encryption,
  member MatrixRTC permissions, audio-only controls, ring suppression and
  conversation send guards. The new upstream quoted-reply API also rejects
  messages in voice channels.
- Ported Home/space routing for DMs and channel creation. Archived conversations
  retain upstream Home routing. Joined voice rooms whose parent is no longer
  available remain reachable in Home, including after a failed parent link.
  Space unread badges exclude DMs that are routed exclusively to Home.
- Retained separate subspace navigation entries as requested. The local
  collapsible groups/mosaic, expansion state and grouping tests were omitted.
  Joined-only space tiles and cached descendant links through departed spaces
  are retained; these affect membership projection, not visual grouping.
- Ported participant-avatar fallback, rounded images, aligned rail markers,
  compact macOS title bar, inline composer, typing row above the composer and
  removal of the global beginning-of-conversation status.
- Preserved upstream reply composition/cancellation, thread attachment APIs,
  selection formatting/context menus, read receipts, dialog focus fields,
  screen picker, speaker following and updater actions when resolving overlaps.
- Included the local icon assets and macOS bundle integration, plus automatic
  successful-build artifact cleanup with one retained build per command.
  Scheme-specific accents were already present upstream and were not reapplied.

## Verification

Validation uses the installed Rust toolchain with the user's explicit approval
because Nix is unavailable.

- `cargo check --workspace --all-targets --locked` passed.
- `cargo fmt --all -- --check` passed.
- `cargo clippy --workspace --all-targets --locked -- -D warnings` passed.
- `cargo test --workspace --locked` passed: 226 passed, 39 ignored (app 105,
  core 73 with 38 ignored, media 18, platform 30 with one ignored; doc tests
  also passed). The initial sandbox library run could not bind four loopback
  listeners; the final combined run passed with loopback access enabled.
- Shell/Python syntax, bundle icon metadata/header and `git diff --check` passed.
- Confirmed original changed files still match the preserved pre-port snapshot.

Regression coverage includes channel parent validation, voice room kind,
DM/Home routing, orphaned voice-room reachability, cached left-space links and
participant avatars. The isolated voice fixture also covers the newly added
quoted-reply guard but remains ignored without fixture services.

## Remaining acceptance

Isolated-server voice/space regressions require the existing fixture services;
no live Matrix or Element checks were performed. Voice-channel first join,
rejoin, member removal and real audio/E2EE interoperability remain unverified.
No physical macOS UI, keyboard/VoiceOver, media permissions or Linux/Windows
runtime acceptance is claimed. No remote publication or merge was performed.

## Develop integration

At the user's request, integrated `upstream/develop` at `b41dff7` before
publication. Retained its navigation-history restoration and dialog behavior,
applied channel routing to the shared history visibility helper, and kept the
compact title bar. Formatting, strict workspace/all-target Clippy and the full
workspace tests passed again (226 passed, 39 ignored).
