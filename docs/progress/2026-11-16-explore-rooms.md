# Explore public rooms (#151)

## Built

- **Explore Public Rooms…** in the account menu opens an Explore Rooms page.
- A row of servers: your own homeserver first, then matrix.org, mozilla.org,
  gnome.org and kde.org. Choosing one loads its public room directory.
- A search field filters by name or topic as you type (the backend waits
  250 ms and drops superseded queries).
- Each room shows its picture, name, member count ("11,904 members") and
  topic, with **Join**, **Ask to Join** (knock rooms), **Open** (already
  joined) or a disabled **Invite only**. Joining goes through the existing
  directory membership path, via the chosen server. **Show More** loads the
  next page.
- The public directory backend and core already existed (marked as unused
  UI contracts); this is its first view. Core now exports the join rule
  type for the view.
- Preview: `--preview-explore` (sample rooms).

## Verified

- Compile: strict Clippy. Offline tests: 202 app tests, including Explore
  listing rooms, switching servers and opening a joined room, and member
  count grouping.
- Local Synapse: `run.sh directory` passes.
- Capture of the preview reviewed.

## Not verified

- Browsing matrix.org and the other servers from a live account (their
  directories are reached through your homeserver's federation); joining a
  room on another server.
