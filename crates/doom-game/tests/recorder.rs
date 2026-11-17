//! The session recorder: a command is stored only as the re-read of its own demo bytes and
//! inside wire 24's domain, the stored copy is what is executed, the file holds exactly those
//! bytes, and the cap is 1,048,576 tics.

use doom_game::app::{RecordError, Recorder, TIC_CAP, wire24_admits};
use doom_game::input::{RecordedTic, read_ticcmd_bytes, write_ticcmd_bytes};
use doom_sim::TicCommand;

fn tic(built: TicCommand) -> RecordedTic {
    let demo = write_ticcmd_bytes(&built);
    RecordedTic {
        built,
        demo,
        executed: read_ticcmd_bytes(demo).expect("not the marker"),
    }
}

fn cmd(forward: i8, side: i8, angle_turn: i16, buttons: u8) -> TicCommand {
    TicCommand {
        forward,
        side,
        angle_turn,
        buttons,
    }
}

#[test]
fn the_stored_command_is_the_reread_and_the_file_holds_its_bytes() {
    let mut r = Recorder::new(4, 3, TIC_CAP);
    let tics = [
        tic(cmd(25, -24, 640, 1)),
        tic(cmd(-50, 40, -320, 2)),
        tic(cmd(0, 0, 32767, 4 | 3 << 3)),
        tic(cmd(0, 0, -129, 0)),
    ];
    let mut bytes = vec![110, 3, 1, 4, 0, 0, 0, 0, 0, 1, 0, 0, 0];
    for t in tics {
        let executed = r.record(t).expect("recorded");
        assert_eq!(executed, t.executed);
        assert_eq!(executed.angle_turn & 0xff, 0);
        assert_eq!(write_ticcmd_bytes(&executed), t.demo);
        bytes.extend(t.demo);
    }
    bytes.push(0x80);
    assert_eq!(r.bytes().expect("written"), bytes);
    assert_eq!(r.commands()[0].angle_turn, 768);
    assert_eq!(r.commands()[2].angle_turn, -32768);
}

#[test]
fn a_changed_or_inadmissible_command_is_never_stored() {
    let mut r = Recorder::new(1, 2, TIC_CAP);
    let mut changed = tic(cmd(25, 0, 640, 0));
    changed.executed.angle_turn = 640; // the full-resolution turn, not the re-read
    assert_eq!(r.record(changed), Err(RecordError::NotReread));
    let mut changed = tic(cmd(25, 0, 0, 0));
    changed.executed.forward = 26;
    assert_eq!(r.record(changed), Err(RecordError::NotReread));
    let mut changed = tic(cmd(25, 0, 0, 0));
    changed.demo[0] = 0x80; // the marker
    assert_eq!(r.record(changed), Err(RecordError::NotReread));
    for bad in [
        cmd(0, i8::MIN, 0, 0),
        cmd(0, 0, 0, 0x80), // BT_SPECIAL
        cmd(0, 0, 0, 0x40),
        cmd(0, 0, 0, 2 << 3), // weapon bits without BT_CHANGE
    ] {
        assert!(!wire24_admits(&bad));
        assert_eq!(r.record(tic(bad)), Err(RecordError::NotAdmitted), "{bad:?}");
    }
    assert!(r.is_empty());
    assert!(wire24_admits(&cmd(127, -127, -32768, 1 | 2 | 4 | 7 << 3)));
    assert!(!wire24_admits(&cmd(i8::MIN, 0, 0, 0)));
}

#[test]
fn the_cap_is_1_048_576_tics() {
    let mut r = Recorder::new(1, 2, TIC_CAP);
    let t = tic(TicCommand::default());
    for _ in 0..TIC_CAP {
        r.record(t).expect("below the cap");
    }
    assert!(r.full());
    assert_eq!(r.record(t), Err(RecordError::Cap));
    assert_eq!(r.len(), 1_048_576);
    assert_eq!(r.bytes().expect("written").len(), 13 + 4 * 1_048_576 + 1);
    let mut small = Recorder::new(1, 2, 1);
    assert!(!small.full());
    small.record(t).expect("one");
    assert_eq!(small.record(t), Err(RecordError::Cap));
}
