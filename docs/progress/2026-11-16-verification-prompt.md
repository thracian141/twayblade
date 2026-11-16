# Verification prompt clears once verified (#147)

## Fixed

- After an emoji (SAS) verification finished, core first downloaded the
  backed-up keys of every encrypted room and only then re-read whether this
  device is verified. With many rooms that took minutes, so the verification
  notice and Settings › Security kept asking to verify.
- A verification that finished on the request level (`Done` without our SAS
  state) never re-read the verified state at all.
- Both paths now re-check with the server at once, and again after 2, 5 and
  10 seconds until the other device's signature arrives, then download keys.

## Verified

- Compile: strict Clippy for `twayblade-core`.
- Local Synapse: `run.sh security` (`secure_backup_fresh_store_recovery_and_sas`)
  passes; it verifies a fresh device by SAS and waits for the owner signature.

## Not verified

- The reported case on the owner's account (the diagnostics log of that
  session had no verification entries).
- Verification started from Element on another device.
