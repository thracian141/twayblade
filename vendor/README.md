# LiveKit WebRTC playback gain patch

Sources: crates.io libwebrtc 0.4.0 and webrtc-sys 0.3.53 (Apache-2.0;
upstream notices retained). Cargo patches keep the existing locked versions.

The only functional changes expose `RtcAudioTrack::set_playback_volume(f64)`
through CXX to WebRTC `AudioTrackInterface::SetVolume`. The bridge rejects
nonfinite values and values outside 0–2. It controls the remote source playout
gain, never publication mute or microphone gain. Twayblade applies it only to
remote tracks. No codec, encryption or transport changes.

Remove both patches when the upstream Rust SDK exposes remote playback gain.

# GPUI Kit menu size patch

Source: crates.io gpui-component 0.9.1 (Apache-2.0; upstream notice kept),
without its integration tests. Cargo patches keep the locked version.

The only changes are in `src/menu/popup_menu.rs` (marked "Twayblade patch"):
menu items use the body text size (1 rem, 13 pt in Twayblade) on 1.75 rem
rows, and their icons are `small` instead of `xsmall`. Upstream used
`text_sm` on 1.625 rem rows, which read at about 11 pt (#149).

Remove the patch when GPUI Kit offers a menu text size option.
