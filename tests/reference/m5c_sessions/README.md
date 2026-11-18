# M5c sessions

Scripted sessions for `tools/reference/m5c_session.py` (M5c step 5c; design in
[docs/M5C_REFERENCE_INVESTIGATION.md §4.7.1](../../../docs/M5C_REFERENCE_INVESTIGATION.md)). The scripts were
written by hand: they are not sessions a person played. Each one is a list of platform-level events with
timestamps in milliseconds on a fake clock, in the format of the M5b sessions (`key`, `motion`, `button`,
`focus`, `end`; `crates/doom-game/examples/m5c_session.rs`), here with a frame period of 10 ms. The tool plays
them through the app under the new contract (wire 25, the sound session, the display with sound) and compares
every tic with linuxdoom-1.10 built with its in-process sound code (one define removed), replaying the
session's demo, under the declared schedule. The four M5b scripts of `tests/reference/m5b_sessions` are played
the same way, unchanged.

Turns are made with the mouse, because a mouse turn is exact at demo resolution: a motion of 256 posts 1024
(the event layer scales by four), which turns by 8192 (an eighth of a full turn) and is recorded as 32 (256 to a full turn).

| Session | Room | What it shows |
| --- | --- | --- |
| `fight` | `m5c_wads/sndmonst.wad` | three pickups, a door (doropn), the rocket launcher fired through the door at zombiemen, shotgun guys and imps (their see, pain, death and attack sounds, explosions, starts refused for want of a channel), the player killed (pldeth), then use asking for the reborn, which ends the session |
| `switch` | `m5c_wads/sndbtn.wad` | a lift switch near the map's origin: the press (swtchn, button slot 0 free), the lift (pstart, pstop) and the release BUTTONTIME = 35 tics later (`linuxdoom-1.10/p_spec.h:247`) from the button slot's origin (`p_spec.c:1151`); the far switch, whose release is out of earshot; a door opening and, VDOORWAIT = 150 tics after it is open (`p_spec.h:365`), closing (doropn, dorcls, `p_doors.c:84`) |
| `pause` | `m5c_wads/sndbusy.wad` | the locked door used twice (oof with no origin, `p_doors.c:228-253`), nine doors opened at once, the app's pause in the tic after their starts and before the buffer that mixes them, the resume, the floor's sounds refused while the doors hold the channels, a pistol shot. Its `#! nopause` line makes the tool also play it without the pause and require the same streams |
| `restart` | `m5c_wads/sndmonst.wad` | pickups and a rocket that sets off the barrel, the app's pause, a restart from the pause (a fresh process, decision T15: a second session with its own demo, compared with its own genuine run), then the use key against a wall (noway, `p_map.c:1104`) and pistol shots |
| `death` | `m5c_wads/sndmonst.wad` | the soulsphere (health 200) and the rocket launcher, then rockets into a wall at point blank while pressing against it: plpain, then a blast that gibs the player (health below -spawnhealth, `linuxdoom-1.10/p_inter.c:719-722`; `A_XScream` starts sfx_slop, `p_enemy.c:1572-1574`), then use asking for the reborn, which ends the session |

Frozen tics (while paused) never appear in a demo; a tic the simulation refuses (the reborn) is not in it
either. The expected results are in `tests/reference/m5c_session_results.json` (counts and digests of the
genuine side).
