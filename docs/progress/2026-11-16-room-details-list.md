# Room details as a vertical list (#155)

## Built

- Info › About now starts with the room's picture (64 pt), its name as a
  heading, an **Encrypted** badge (or the unencrypted warning) and the
  topic, centred.
- Below, actions are rows with an icon and a label, grouped by separators:
  - **Favourite** and **Mute notifications** with switches; **Invite**.
  - **People**, **Pinned messages** and **Files** with their counts; People
    and Files open their tabs.
  - **Copy link**, then the notification level control.
  - **Leave room…** (or **Leave conversation…**) in red, opening the leave
    confirmation.
- The room ID and its copy button stay at the bottom. The About / Members /
  Files tabs are unchanged.
- Replaces the row of four small tiles.

## Verified

- Compile: strict Clippy. Offline tests: 198 app tests pass, including the
  unencrypted warning wrapping inside the panel.
- Capture of `--preview-info` reviewed.

## Not verified

- Threads, pinned-message list, room settings, export and report have no
  views yet, so they have no rows; VoiceOver reading of the switches.
