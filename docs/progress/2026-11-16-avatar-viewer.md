# Profile pictures open full size (#148)

## Built

- In the profile panel, a person's picture (when they have one) is a button:
  "View profile picture of <name>". It opens the picture over the window at
  up to 400 pt, with the name and a Close button in the toolbar, like the
  image viewer. Escape, a click, the Close button or the mouse Back button
  close it; focus returns to the window. Initials without a picture stay
  inert.
- The picture is fetched once at 800 px through the same bounded avatar
  decoder (square crop) and its GPU image is released on close. Only
  `mxc://` URLs open.

## Verified

- Compile: strict Clippy.
- Offline UI test: opening shows the dialog (offline the picture reports it
  couldn't load instead of spinning), Close removes it, and a non-Matrix URL
  doesn't open.

## Not verified

- A real picture on a live account (the offline preview has no media).
- VoiceOver reading the button and dialog.
