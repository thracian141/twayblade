# People you know in Invite and Start a Direct Message (#150)

## Built

- Under the Matrix ID field, both dialogs list up to six **People you know**:
  the partners of your direct conversations, most recent first, with picture,
  name and Matrix ID. Typing filters them by name or ID.
- In **Invite People**, picking a person fills the field (people already in
  the room are left out); Send Invitation then invites them.
- In **Start a Direct Message**, picking a person reopens your existing
  conversation with them instead of creating a new one.
- Each row has an accessible label ("Invite Sam (@sam:…)", "Open your
  conversation with Sam") and the list is a labelled group.

## Verified

- Compile: strict Clippy. Offline tests: a unit test for the list (recency
  order, one entry per person, filter by name and ID, room members excluded,
  limit) and a UI test that picking Alex in Start a Direct Message closes the
  dialog and opens the conversation. 200 app tests pass.
- Capture of `--preview-dm` reviewed.

## Not verified

- A live account. People you share rooms with but never messaged are not
  listed yet (their membership is loaded per room); server-side user
  directory search is a separate step.
