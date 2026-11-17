# M5b input scripts and goldens

These files are the evidence for M5b's input path: event scripts and the commands the genuine
linuxdoom-1.10 input code builds from them. The goldens come from the C only, never from Rust output.

- `scripts/*.txt` are written by [`m5b_input_scripts.py`](../../../tools/reference/m5b_input_scripts.py).
  Its docstring gives the name prefixes. `ev_`, `s6_`, `modern_`, `round_`, `sens_` and `dclk_` scripts
  work at the `event_t` level, which is what the app posts. `x_` scripts go through linuxdoom's own X
  layer. `s5_` scripts show what the genuine responders do with the keys the app never forwards.
- `goldens/*.golden` hold one file per script, from the strict build. The overflow and address builds
  must write the same bytes, and so must the optional 32-bit build.
  [`m5b_input_reference.py`](../../../tools/reference/m5b_input_reference.py) writes them, and its
  `--check` compares them.
- `witness/*.txt` are scripts outside the app's event domain. They are not goldens. The reference tool
  only checks that the overflow build stops on them.

The driver, its contract and the design are described in
[`m5b_input_driver.c`](../../../tools/reference/m5b_input_driver.c),
[`m5b_input_contract.json`](../m5b_input_contract.json) and
[section 2.2 of the M5b investigation](../../../docs/M5B_REFERENCE_INVESTIGATION.md#22-input-the-genuine-event-path-as-a-headless-driver).

## Script format

A script has one directive per line. Lines starting with `#` are comments.

| Directive | Meaning |
| --- | --- |
| `config set NAME VALUE` | One row of the configuration file that the genuine `M_LoadDefaults` reads. Integers only. With no such line, the genuine default table applies. |
| `config record 0` | Turns recording off. Recording is on by default. |
| `config grab 1`, `config warpmodel 1` | X level only: linuxdoom's `-grabmouse`, and the fake server's answer to a pointer warp. |
| `ev keydown N`, `ev keyup N` | Posts a key `event_t` with `data1 = N` (a DOOM key code, for example `0xad` for the up arrow). |
| `ev mouse B DX DY` | Posts an `ev_mouse` with `data1 = B`, `data2 = DX` and `data3 = DY`. |
| `press K`, `release K`, `tap K`, `bpress B`, `brelease B`, `motion X Y`, `expose` | X events for the next tic. `tap` is a press and a release in the same pump. |
| `tic [N]` | Runs N tics (default 1). Each tic delivers the queued events, builds a command and runs `G_Ticker`. |
| `loadlevel` | Calls `G_DoLoadLevel` between tics. |
| `enddemo` | Calls `G_CheckDemoStatus`, which writes the demo and ends the run. |

An `ev` line posts its event at once. Events posted between two `tic` lines are delivered in the
later tic, before that tic's command is built.

## Golden format

A golden is plain text with one row per line. The fields are separated by single spaces, and every
number is a signed decimal unless stated otherwise. A Rust test can read it with `split_whitespace`
and `parse::<i32>`, with no new dependency.

| Row | Fields | Meaning |
| --- | --- | --- |
| `h` | `HEX` | The 13 demo header bytes, in hex. Present only when the script records. |
| `s` | `TIC STATE` | The observed state vector, as comma-separated `name=value` pairs. It is printed at start (`TIC` = -1) and whenever it changes. |
| `L` | `TIC` | A `loadlevel` directive ran before tic `TIC`. |
| `e` | `TIC TYPE D1 D2 D3 WHO` | An event delivered by `D_ProcessEvents` in tic `TIC`, in delivery order. `TYPE` is 0 for a key down, 1 for a key up and 2 for the mouse. Key events print `D2` and `D3` as `-`. |
| `q` | `TIC POSTED DELIVERED` | Printed only when the two counts differ, which means events were lost to the 64-entry event ring. |
| `t` | `TIC F S A B XF XS XA XB DEMO TH DCT DCS DCL DCT2 DCS2 DCL2 MB KEYS` | One row per tic. The fields are listed below. |
| `r` | `N` | The previous `t` row repeats `N` more times, with its tic advancing by one each time. It always follows a `t` row directly. |
| `c` | `TIC C H XC XH` | The built and executed `consistancy` and `chatchar`, printed only when one is nonzero. No golden has one: when no `c` row is present, all four are 0. |
| `x` | `TIC KIND MESSAGE...` | `I_Error` or `I_Quit` stopped the run in tic `TIC`. That tic has no `t` row. |
| `z` | `TIC DEMOLEN` | The script ended normally before tic `TIC`. `DEMOLEN` is the number of demo bytes written so far, or -1 when not recording. |

The `t` row fields are:

- `F S A B`: the command `G_BuildTiccmd` built, before the demo round trip. These are `forwardmove`,
  `sidemove`, `angleturn` at its full 16 bits, and `buttons`.
- `XF XS XA XB`: the command the game executes, which is `players[0].cmd` after `G_Ticker`. While
  recording, `G_Ticker` writes the command to the demo and reads it back, so `XA` is a multiple of 256
  (see S3). With `config record 0`, the executed command equals the built one.
- `DEMO`: the four bytes written to the demo for this tic, in hex, or `-` when not recording.
- `TH`: `turnheld`, capped at 6. Values from 6 up print as 6, because `G_BuildTiccmd` reads it only
  through `turnheld < 6`.
- `DCT DCS DCL`: `dclicktime`, `dclickstate` and `dclicks` of the forward-button double click.
  `DCT2 DCS2 DCL2` are the same for the strafe button. Both timers are capped at 21, because only
  `> 1` and `> 20` are read. `DCS` holds the raw value it was given, so it can be 4.
- `MB`: a bitmask of `mousearray[0..3]`. Bit 0 is `mousebuttons[-1]`, which is never set. Bit 1 is
  fire, bit 2 is strafe and bit 3 is forward.
- `KEYS`: the hex codes of the keys `gamekeydown` holds down, comma-separated, or `-` when none.

`F S A B` are read straight after `G_BuildTiccmd`. Every other `t` field is read after that tic's
`G_Ticker`. The `e` rows of a tic come before its `t` row, because events are delivered before the
command is built.
