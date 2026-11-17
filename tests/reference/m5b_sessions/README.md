# M5b sessions

Scripted sessions for `tools/reference/m5b_session.py` (M5b step 7; design in
[docs/M5B_REFERENCE_INVESTIGATION.md §2.3](../../../docs/M5B_REFERENCE_INVESTIGATION.md#23-sessions-replaying-a-recorded-session-through-both-engines)).
The scripts were written by hand: they are not sessions a person played. Each one is a list of
platform-level events with timestamps in milliseconds on a fake clock (`key`, `motion`, `button`,
`focus`, `end`; the format is in `crates/doom-game/examples/m5b_session.rs`), played through the
app's own code path and then replayed through the genuine engine. Frames are compared under the
declared sound-free, wipe-free contract (decision S15).

| Session | Room | Tics | What it shows |
| --- | --- | --- | --- |
| `amp_walk` | `m5a_wads/amp.wad`, screenblocks 10 | 75 | keyboard turning (320 and 640 recorded as 256 and 768), mouse turning with sub-256 turns rounded or dropped, a mouse and a keyboard fire, a pickup with its message and bonus palette, a pause with Esc in the middle |
| `palbonus_pickups` | `m5b_wads/palbonus.wad`, screenblocks 11 | 145 | four pickups with their messages and bonus palettes 10 to 12, mouse turns lost to demo resolution, a pause on focus loss resumed with Pause |
| `palred_damage` | `m5b_wads/palred.wad`, screenblocks 10 | 155 | a soulsphere pickup, bonus palette 10, a barrel shot point-blank: damage, red palettes 2 to 8 and the face of self-inflicted damage (index 7, `linuxdoom-1.10/st_stuff.c:855-871`), a pause in the middle, a fast keyboard turn |
| `exits_secret` | `m4a_wads/exits.wad`, screenblocks 10 | 61 | the M4a EXIT2 walk made with the mouse and keyboard, ending the level at the secret exit |

Frozen tics (while paused) never appear in a demo. The expected results are in
`tests/reference/m5b_session_results.json` (counts and digests of the genuine side).
