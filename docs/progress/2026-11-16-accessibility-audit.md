# Accessibility tree fixes from the #51 audit

## Audit

On 2026-11-16 the accessibility tree of the 0.1.1-rc.1 release app was read
through the macOS Accessibility API (System Events), read-only. Every button
and field had a name; messages read as "sender, time, text" with reply and
edit notes; regions (Spaces, Conversations, Favourites, Rooms) were labelled.

## Fixed here

- Each sender run exposed the avatar ("Open profile of X") and the name ("X")
  as two buttons for the same action. The avatar is now presentational and
  not a Tab stop; the name button is labelled "Open profile of X".
- The message list (a log region) is named "Messages in <room>".
- The open room's name in the header is a heading.

## Not a problem

- Room rows already include unread counts, mentions and "notifications
  muted" in their labels (tested in `room_rows_read_name_selection_and_attention`);
  the audited rooms had none.

## Still open

- Hover actions are 3–4 buttons per message in the tree (#156).
- Keyboard-only order (Tab, F6, Escape) through every view: needs a run with
  the release app closed, since both processes are named `twayblade` and
  System Events resolves them by name.
- VoiceOver speech itself, Orca and Narrator.

## Verified

- Compile: strict Clippy. Offline tests: 197 app tests pass.
- The changed labels were not re-read through System Events (same name
  clash).
