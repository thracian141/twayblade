# Set a status message (#153)

## Built

- The account menu gains **Set Status Message…** (or **Change Status
  Message…** when one is set), with an icon, above Clear Status Message.
- It opens a small dialog titled "Status Message": a line explaining who
  sees it, a text field that starts on the current status, a character count
  against core's 256-character limit (red when over), **Save** and, when a
  status is set, **Clear**. Saving keeps the current presence (Online, Away,
  Invisible) and closes the dialog.
- Core already validated and sent `status_msg` with presence; only the view
  was missing.

## Verified

- Compile: strict Clippy.
- Offline UI test: the dialog opens on the current status with Save and
  Clear.

## Not verified

- Setting a status on a live account and seeing it in Element.
