# Message previews in the room list (#154)

## Built

- Each conversation in the sidebar shows the latest message on a second
  line: "Sender: text" in rooms, the text alone in direct messages,
  "You: …" for your own, and "Image", "File", "Encrypted message" and so on
  for other kinds (the same wording as the notifications popover).
- Two-line rows are taller (40/46/52 pt by sidebar density) with a 30 pt
  picture, or a round # / lock tile for rooms without one. Unread rooms show
  the preview in the normal text colour, others muted; the row's accessible
  label ends with the preview.
- Settings › Appearance › **Show message previews** turns it on and off
  (on by default). Off restores the one-line rows.
- Previews come from core's `room_previews` (the SDK's latest event per
  room, local, no extra requests); the sidebar asks for at most 500 rooms
  and asks again only when a room's activity changes.

## Verified

- Compile: strict Clippy. Offline tests pass.
- Capture of the offline preview reviewed (sample previews).

## Not verified

- A live account: how quickly previews fill in for rooms the SDK has not
  tracked yet (the first request starts tracking; the preview may appear on
  the next activity).
