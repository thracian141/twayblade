# Search opens like Spotlight (#152)

## Built

- Focusing the title-bar search (click or ⌘K) opens a panel under it.
  - Empty: **Recently viewed** (the conversations you opened this session,
    newest first; before any, the most active ones) as a row of pictures,
    and **Search for**: Public rooms (Explore Rooms), People (Start a Direct
    Message) and a hint that typing searches messages.
  - Typing: **Conversations** whose names match (names starting with the
    text first, up to six, with topic or Matrix ID), then **Messages
    containing "…"** and **Public rooms matching "…"** (Explore Rooms with
    the text filled in).
- ↑ ↓ move the selection, Return opens it (the first row by default),
  Escape closes and clears. Choosing a conversation clears the field.
  Message search keeps the text and moves focus to the results.
- The sidebar still filters while you type; its separate "Search messages"
  row moved into the panel.
- Preview: `--preview-spotlight`.

## Verified

- Compile: strict Clippy. Offline tests: 207 app tests, including the panel
  showing recent conversations, typing "crit" and Return opening Critique,
  and the existing message-search flow.
- Capture of the preview reviewed.

## Not verified

- A live account; recently viewed is per session (not saved yet); VoiceOver
  reading of the panel rows.
