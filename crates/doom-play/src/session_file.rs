//! The session demo on disk (M5d): created only with its content, kept current tic by tic, and
//! meant to be a valid demo at every instant, so a session killed at any point (Ctrl+C, a closed
//! console window, a lost display connection, `SIGKILL`) leaves a file that replays a prefix of
//! the session. The argument below covers local file systems whose buffered writes go through the
//! page cache (ext4, tmpfs on Linux); it does not cover FUSE, NFS or 9p (WSL's `/mnt/c`), the
//! fallback claim's window on file systems without hard links, or a write that keeps failing (the
//! file then stays at its last good state and falls behind).
//!
//! **Who calls these operations.** In play every one of them is made by the session writer's
//! thread ([`crate::session_writer`]), in the order the app asked for them, so the loop never waits
//! for the disk (the owner's ruling of 2026-11-20); the synchronous [`crate::FsHost`] makes them on
//! the caller's thread (tests). Each operation is the same all-or-nothing operation either way;
//! what the thread changes is the lag: the file holds at least every tic of the writer's last
//! completed write and at most every tic run, and a kill loses the tics still queued (before, the
//! file held every tic run or every tic but the last). The writer appends several waiting tics
//! through one open ([`append_tics`]: still one five-byte write per tic, each inside one page). The
//! flush to the disk before a rename stays where M5b had it (the app's own whole saves, and the
//! claim); the writer's own catch-up and page-boundary rewrites are not flushed ([`save_with`]),
//! because a kill of the process never loses what reached the page cache.
//!
//! - **Creation** ([`claim_kind`]): the first content (the 13-byte header and the end marker) is
//!   written to a new temporary file (`NAME.PID.claim.tmp`), flushed, and hard-linked to the
//!   session's name, which fails if the name exists (so no file is ever replaced); then the
//!   temporary name is removed. The name therefore never appears empty or partial. A kill between
//!   the link and the removal leaves a `*.claim.tmp` file beside the demo (never a `.lmp`); so
//!   does a removal that fails, and that second name of the session's file is never written
//!   again, because the whole rewrite uses its own temporary name (`NAME.PID.tmp`) and creates
//!   it new. Where the file system refuses the hard link (FAT, exFAT, some network shares), the
//!   claim falls back to M5b's exclusive create (`create_new`) and a write, which leaves one
//!   window: a kill between the create and the write leaves an empty `.lmp`.
//! - **Every tic** ([`append`]): the tic's four demo bytes and a new marker, `DEMOMARKER` (0x80,
//!   linuxdoom-1.10 g_game.c:1488), go at the old marker's offset in **one** `write` of five
//!   bytes, overwriting the old marker and extending the file by four bytes. The original needs
//!   the marker: `G_ReadDemoTiccmd` tests a tic's first byte for it (g_game.c:1493) and ends the
//!   playback there (`G_CheckDemoStatus`, g_game.c:1496); without it, it goes on reading past the
//!   end of the demo lump, which `W_CacheLumpNum` allocated at exactly the lump's length
//!   (w_wad.c:490), into whatever memory follows. The recording engine writes it the same way,
//!   once, when recording ends (g_game.c:1679-1680). doom-sim's reader refuses both a missing
//!   marker and any byte after it, so of the two-step orders (tic, then marker; or marker, then
//!   tic) each leaves an instant at which the file is not a valid demo; only one write that lands
//!   all or nothing keeps it valid at every instant. A recorded tic's forward byte is never 0x80
//!   (wire 24 refuses forward −128), so the reader never stops early.
//! - **Why one five-byte write lands all or nothing under `SIGKILL` on Linux** (this rests on a
//!   reading of the kernel's source, not on any promise of POSIX and not on a test): a buffered
//!   write to a regular file on ext4 or tmpfs goes through `generic_perform_write` (Linux v6.18
//!   mm/filemap.c:4227-4318; ext4 fs/ext4/file.c:299, tmpfs mm/shmem.c:3466), which copies the
//!   data one page-cache folio at a time and checks for a pending fatal signal only at the top of
//!   each folio's iteration
//!   (mm/filemap.c:4250-4253), before the folio is touched; the copy into the folio and the size
//!   update (`write_end`) of that iteration complete before the next check. Folios are at least
//!   one page and naturally aligned, so a write that stays inside one 4096-byte-aligned block
//!   stays inside one folio and one iteration (for any page size of 4096 bytes or more), and
//!   `SIGKILL` either prevents it or lets it finish. A write that straddles a 4096-byte boundary
//!   can be cut between its two folios. The marker's offset is 13 + 4·N, which is 1 mod 4, so
//!   the five bytes straddle a boundary exactly when the offset is 4093 mod 4096 (once every
//!   1,024 tics): there [`append_fits_one_page`] says no. In play the session writer then
//!   rewrites the file whole itself, unsynced, through [`save_with`] (a new temporary file and a
//!   rename, atomic by the rename); the synchronous [`crate::FsHost`]'s [`append`] declines and the
//!   app rewrites the file whole through [`save`] (the same, synced). The one remaining way the
//!   kernel loop can stop inside a folio is a short copy from
//!   the caller's buffer (a fault on the five source bytes, which the app has just written and
//!   which are resident): not expected.
//! - **What the tests show.** Deterministic unit tests pin what the code does: one `write` call
//!   of the five bytes per append ([`write_tail`]), a whole rewrite that replaces the file (a new
//!   file renamed over it, never the old one written), and a claim whose name appears as the
//!   temporary file (the hard link) or, where the link fails, through the fallback. The kill test
//!   (`kill_tests`) and `tools/reference/m5d_kill_check.py` are evidence for the steady state: no
//!   gross fault (a file that loses its marker, replays differently, holds fewer tics than the
//!   writer had reported written or more than were run, or lags the game by more than the kill
//!   test's lag bound, three times the writer's queue bound, which a slow disk can exceed without
//!   a fault) in the kills they make. The kill test's self-test shows that it catches a torn append whose two
//!   writes are a millisecond apart; a kill cannot be aimed inside a window of microseconds, so
//!   neither is evidence for the all-or-nothing property itself, which rests on the kernel as
//!   above.
//! - **Elsewhere** (inference, untested here): macOS's kernel is not Linux, and the macOS CI run
//!   of the kill test is its only evidence. On Windows, a cached `WriteFile` of five bytes within
//!   one page is expected to be copied into the cache in one piece before `TerminateProcess` (or
//!   the console's close, which ends the process the same way after its handler times out) can
//!   take effect, but this is not verified; the page rule and the whole-rewrite path are the same
//!   there.
//!
//! The file is opened for each append and closed after it (no handle is kept): at most 35 opens a
//! second, on the writer's thread; no handle can be left open across the rename of a whole
//! rewrite (Windows refuses to rename over an open file), and [`crate::FsHost`] keeps its fields.
//!
//! **Leftovers** (the owner's ruling 6): a kill during a claim or a whole rewrite can leave
//! `NAME.PID.claim.tmp` or `NAME.PID.tmp`; at start, doom-play removes the files of exactly those
//! two name patterns that are more than an hour old ([`remove_leftovers`]) and prints the count.

use std::io::{Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

/// The size of the blocks a five-byte append must not straddle: the smallest page size of the
/// page cache (4096 bytes on x86-64 and the default on arm64; a larger page only makes the rule
/// conservative).
pub(crate) const PAGE: u64 = 4096;

/// The bytes of one append: a tic's four demo bytes and the end marker.
pub(crate) const TAIL: u64 = 5;

/// Whether `len` bytes written at byte `at` stay inside one [`PAGE`]-aligned block, so that one
/// buffered write of them lands all or nothing under `SIGKILL` on Linux (see the module
/// documentation). An empty write is trivially inside.
pub(crate) fn append_fits_one_page(at: u64, len: u64) -> bool {
    len == 0 || at / PAGE == (at + len - 1) / PAGE
}

/// The whole rewrite's temporary name beside `name` for this process (two processes never share
/// it): `NAME.PID.tmp`.
fn temporary(dir: &Path, name: &str) -> PathBuf {
    dir.join(format!("{name}.{}.tmp", std::process::id()))
}

/// The claim's temporary name beside `name` for this process: `NAME.PID.claim.tmp`, never the
/// whole rewrite's ([`temporary`]). After the hard link it is a second name of the session's file
/// until it is removed; if that removal fails, a later [`save`] still writes its own temporary
/// file and never this one.
fn claim_temporary(dir: &Path, name: &str) -> PathBuf {
    dir.join(format!("{name}.{}.claim.tmp", std::process::id()))
}

/// Opens a new, empty file at the temporary name `tmp`: any stale file of that name is removed
/// first and the file is created exclusively (`create_new`), so a write never goes through an
/// existing file or a second link to one (a removal that failed makes the create fail instead).
fn fresh(tmp: &Path) -> std::io::Result<std::fs::File> {
    let _ = std::fs::remove_file(tmp);
    std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(tmp)
}

/// Creates `dir/name` holding `bytes`, never replacing an existing file: `Ok(false)` when the
/// name is taken (that file is left alone). `bytes` go first to a temporary file through `write`
/// (doom-play writes and flushes them to the disk), which is then hard-linked to the name, so the
/// name appears only with its whole content; the temporary name is removed afterwards. When the
/// write fails, the temporary file is removed and the error returned (no file is created, so the
/// retry can take the same name). When the hard link fails for any reason other than a taken
/// name (a file system without hard links), the claim falls back to the exclusive create of M5b
/// ([`claim_create_new`]). doom-play's hosts claim through [`claim_kind`].
#[cfg(test)]
pub(crate) fn claim_new(
    dir: &Path,
    name: &str,
    bytes: &[u8],
    write: impl FnOnce(&mut std::fs::File, &[u8]) -> std::io::Result<()>,
) -> Result<bool, String> {
    claim_with(
        dir,
        name,
        bytes,
        write,
        |tmp, path| std::fs::hard_link(tmp, path),
        |tmp| std::fs::remove_file(tmp),
    )
    .map(|c| c != Claim::Taken)
}

/// `claim_new` as doom-play makes it (the temporary file written and flushed to the disk, the
/// real hard link), saying how it ended.
pub(crate) fn claim_kind(dir: &Path, name: &str, bytes: &[u8]) -> Result<Claim, String> {
    claim_with(
        dir,
        name,
        bytes,
        |f, b| f.write_all(b).and_then(|()| f.sync_all()),
        |tmp, path| std::fs::hard_link(tmp, path),
        |tmp| std::fs::remove_file(tmp),
    )
}

/// The one line printed on standard error when a claim took the fallback (the owner's ruling 5 of
/// 2026-11-20); `path` is the session's file.
pub(crate) fn fallback_line(path: &Path) -> String {
    format!(
        "doom-play: {}: the file system refused a hard link, so the session file was created by \
         the fallback claim (created empty, then written)",
        path.display()
    )
}

/// How [`claim_with`] ended.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Claim {
    /// The temporary file was hard-linked to the name: the name appeared with its content.
    Linked,
    /// The hard link failed (not for a taken name) and M5b's exclusive create made the file
    /// ([`claim_create_new`]).
    Created,
    /// The name was taken; nothing was replaced.
    Taken,
}

/// [`claim_kind`] with the write, the hard link (`link(tmp, path)`) and the removal of the
/// temporary name after it (`remove_tmp(tmp)`) passed in, so that tests can make either fail.
fn claim_with(
    dir: &Path,
    name: &str,
    bytes: &[u8],
    write: impl FnOnce(&mut std::fs::File, &[u8]) -> std::io::Result<()>,
    link: impl FnOnce(&Path, &Path) -> std::io::Result<()>,
    remove_tmp: impl FnOnce(&Path) -> std::io::Result<()>,
) -> Result<Claim, String> {
    let path = dir.join(name);
    let fail = |e: std::io::Error| format!("{}: {e}", path.display());
    std::fs::create_dir_all(dir).map_err(fail)?;
    // a name already taken is left alone before anything is written (the link below checks it
    // again, atomically, for a name taken meanwhile)
    if std::fs::symlink_metadata(&path).is_ok() {
        return Ok(Claim::Taken);
    }
    let tmp = claim_temporary(dir, name);
    let written = fresh(&tmp).and_then(|mut f| write(&mut f, bytes));
    if let Err(e) = written {
        let _ = std::fs::remove_file(&tmp);
        return Err(fail(e));
    }
    let linked = link(&tmp, &path);
    // a failed removal leaves a `*.claim.tmp` name (after a link, a second name of the session's
    // file), which no later write opens: `save` uses its own temporary name
    let _ = remove_tmp(&tmp);
    match linked {
        Ok(()) => Ok(Claim::Linked),
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => Ok(Claim::Taken),
        // no hard links here: M5b's exclusive create, with its window (see the module
        // documentation); the content is written again from `bytes`
        Err(_) => claim_create_new(&path, bytes).map(|created| {
            if created {
                Claim::Created
            } else {
                Claim::Taken
            }
        }),
    }
}

/// M5b's claim, the fallback of [`claim_kind`]: creates `path` exclusively (`create_new`) and
/// writes and flushes `bytes`; `Ok(false)` when the name is taken. A failed write removes the file
/// this call created. A kill between the create and the write leaves an empty file.
fn claim_create_new(path: &Path, bytes: &[u8]) -> Result<bool, String> {
    let fail = |e: std::io::Error| format!("{}: {e}", path.display());
    let file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path);
    match file {
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => Ok(false),
        Err(e) => Err(fail(e)),
        Ok(mut f) => match f.write_all(bytes).and_then(|()| f.sync_all()) {
            Ok(()) => Ok(true),
            Err(e) => {
                drop(f);
                let _ = std::fs::remove_file(path);
                Err(fail(e))
            }
        },
    }
}

/// Rewrites `dir/name` whole: writes a new temporary file ([`fresh`], under the rewrite's own
/// name, [`temporary`]), flushes it to the disk and renames it over the session's file (atomic:
/// the name holds the old or the new content, and the old file is never written); a failed write
/// removes the temporary file.
pub(crate) fn save(dir: &Path, name: &str, bytes: &[u8]) -> Result<(), String> {
    save_with(dir, name, bytes, true)
}

/// [`save`], with the flush to the disk before the rename only when `synced`. The session writer
/// ([`crate::session_writer`]) makes its own catch-up and page-boundary rewrites unsynced: a kill
/// of the process never loses what reached the page cache, so the rename's atomicity is all the
/// kill guarantee needs; a flush only matters for an operating-system crash or a power loss,
/// which this design never covered (the directory is not flushed either).
pub(crate) fn save_with(dir: &Path, name: &str, bytes: &[u8], synced: bool) -> Result<(), String> {
    let path = dir.join(name);
    let tmp = temporary(dir, name);
    let written = fresh(&tmp)
        .and_then(|mut f| {
            f.write_all(bytes)
                .and_then(|()| if synced { f.sync_all() } else { Ok(()) })
        })
        .and_then(|()| std::fs::rename(&tmp, &path));
    if written.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    written.map_err(|e| format!("{}: {e}", path.display()))
}

/// Extends `dir/name` by one tic: writes the five bytes of `tail` at byte `at` in one `write`
/// ([`write_tail`]; see the module documentation). Declines (`Ok(false)`, and the app rewrites the
/// file whole) when the five bytes would straddle a [`PAGE`] boundary, or when the file does not
/// hold exactly `at + 1` bytes (it is not the file the app last wrote; the whole rewrite repairs
/// it). A write that the system shortens is an error: the file may then lack its marker, and the
/// app's next write is the whole rewrite.
pub(crate) fn append(dir: &Path, name: &str, at: u64, tail: &[u8; 5]) -> Result<bool, String> {
    append_tics(dir, name, at, std::slice::from_ref(tail))
}

/// Extends `dir/name` by `tails.len()` tics, the first at byte `at`: [`append`]'s write once per
/// tic, in order, through one open of the file. Each tic is still exactly **one** `write` call of
/// its five bytes at the marker the previous call left ([`write_tail`]), so after each call the
/// file is a valid demo, and a kill between two calls leaves the earlier tics. Declines
/// (`Ok(false)`, nothing written) when any of the writes would straddle a [`PAGE`] boundary
/// (checked before anything is written) or when the file does not hold exactly `at + 1` bytes; a
/// shortened write is an error (the later tics are then not written).
pub(crate) fn append_tics(
    dir: &Path,
    name: &str,
    at: u64,
    tails: &[[u8; 5]],
) -> Result<bool, String> {
    let offsets = (0..tails.len() as u64).map(|i| at + 4 * i);
    if offsets.clone().any(|o| !append_fits_one_page(o, TAIL)) {
        return Ok(false);
    }
    let path = dir.join(name);
    let fail = |e: std::io::Error| format!("{}: {e}", path.display());
    let mut f = std::fs::OpenOptions::new()
        .write(true)
        .open(&path)
        .map_err(fail)?;
    let mut len = f.metadata().map_err(fail)?.len();
    for (o, tail) in offsets.zip(tails) {
        match write_tail(&mut f, len, o, tail).map_err(fail)? {
            // only the first write can find another length: `len` follows each write
            None => return Ok(false),
            Some(5) => len = o + TAIL,
            Some(n) => {
                return Err(format!(
                    "{}: {n} of 5 bytes written; the file is rewritten whole next",
                    path.display()
                ));
            }
        }
    }
    Ok(true)
}

/// The write of [`append`] on the open session file `f`, which holds `len` bytes: `None` (nothing
/// written) unless `len` is `at + 1`; otherwise a seek to `at` and exactly **one** `write` call of
/// the five bytes, whose count is returned as the system gave it (a short count is not retried:
/// a second call would be a second write, which a kill can separate from the first).
fn write_tail<F: Write + Seek>(
    f: &mut F,
    len: u64,
    at: u64,
    tail: &[u8; 5],
) -> std::io::Result<Option<usize>> {
    if len != at + 1 {
        return Ok(None);
    }
    f.seek(SeekFrom::Start(at))?;
    f.write(tail).map(Some)
}

/// How old a leftover temporary file must be before a later run removes it (the owner's ruling 6
/// of 2026-11-20: more than an hour, by its modification time).
pub(crate) const LEFTOVER_AGE: std::time::Duration = std::time::Duration::from_secs(3_600);

/// Whether `name` is one of this module's own temporary names, matched strictly (ruling 6):
/// `NAME.PID.tmp` ([`temporary`]) or `NAME.PID.claim.tmp` ([`claim_temporary`]), where `NAME` is
/// a session demo's name as doom-game builds it (`E1M<1-9>-skill<digit>-<8 digits>-<6
/// digits>.lmp`, or with a suffix `-2` to `-9999` before `.lmp`) and `PID` a process id (decimal
/// digits, no leading zero, at most `u32::MAX`). Nothing else matches.
pub(crate) fn is_leftover_name(name: &str) -> bool {
    let Some(rest) = name.strip_suffix(".tmp") else {
        return false;
    };
    let rest = rest.strip_suffix(".claim").unwrap_or(rest);
    let Some((session, pid)) = rest.rsplit_once('.') else {
        return false;
    };
    let pid_ok = !pid.is_empty()
        && pid.bytes().all(|b| b.is_ascii_digit())
        && !pid.starts_with('0')
        && pid.parse::<u32>().is_ok();
    pid_ok && is_session_name(session)
}

/// Whether `name` is a session demo's name (see [`is_leftover_name`]).
fn is_session_name(name: &str) -> bool {
    let digits = |s: &str, n: usize| s.len() == n && s.bytes().all(|b| b.is_ascii_digit());
    let Some(stem) = name.strip_suffix(".lmp") else {
        return false;
    };
    let Some(rest) = stem.strip_prefix("E1M") else {
        return false;
    };
    let mut parts = rest.split('-');
    let (Some(map), Some(skill), Some(date), Some(time)) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return false;
    };
    let suffix_ok = match parts.next() {
        None => true,
        Some(n) => {
            !n.is_empty()
                && n.bytes().all(|b| b.is_ascii_digit())
                && !n.starts_with('0')
                && n.parse::<u32>().is_ok_and(|n| (2..=9_999).contains(&n))
        }
    };
    let map_ok = map.len() == 1 && (b'1'..=b'9').contains(&map.as_bytes()[0]);
    let skill_ok = skill.strip_prefix("skill").is_some_and(|s| digits(s, 1));
    parts.next().is_none() && suffix_ok && map_ok && skill_ok && digits(date, 8) && digits(time, 6)
}

/// Whether a file last modified at `modified` is more than [`LEFTOVER_AGE`] old at `now` (a
/// modification time in the future is not old).
pub(crate) fn is_stale(modified: std::time::SystemTime, now: std::time::SystemTime) -> bool {
    now.duration_since(modified)
        .is_ok_and(|age| age > LEFTOVER_AGE)
}

/// Removes the leftover temporary files in `dir` (ruling 6): only regular files whose names
/// [`is_leftover_name`] matches and that [`is_stale`] at `now`. Returns how many were removed and
/// the errors met (a missing directory is neither). Run once at start, before the session's
/// first file.
pub(crate) fn remove_leftovers(dir: &Path, now: std::time::SystemTime) -> (u64, Vec<String>) {
    let mut removed = 0;
    let mut errors = Vec::new();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return (0, errors);
    };
    for entry in entries {
        let entry = match entry {
            Ok(e) => e,
            Err(e) => {
                errors.push(format!("{}: {e}", dir.display()));
                continue;
            }
        };
        let name = entry.file_name();
        let Some(name) = name.to_str() else { continue };
        if !is_leftover_name(name) {
            continue;
        }
        let path = entry.path();
        let stale = std::fs::symlink_metadata(&path)
            .and_then(|m| Ok(m.is_file() && is_stale(m.modified()?, now)));
        match stale {
            Ok(false) => {}
            Ok(true) => match std::fs::remove_file(&path) {
                Ok(()) => removed += 1,
                Err(e) => errors.push(format!("{}: {e}", path.display())),
            },
            Err(e) => errors.push(format!("{}: {e}", path.display())),
        }
    }
    (removed, errors)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "doom-play-session-file-{tag}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn names(dir: &Path) -> Vec<String> {
        let mut n: Vec<_> = std::fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().into_string().unwrap())
            .collect();
        n.sort();
        n
    }

    /// The page rule: inside one 4096-byte block or not, and for the demo's offsets 13 + 4·N
    /// exactly the offsets 4093 mod 4096, once every 1,024 tics (the first after 1,020 tics).
    #[test]
    fn the_append_straddles_a_page_exactly_at_offsets_4093_mod_4096() {
        assert!(append_fits_one_page(0, 5));
        assert!(append_fits_one_page(4091, 5));
        assert!(!append_fits_one_page(4092, 5));
        assert!(!append_fits_one_page(4095, 5));
        assert!(append_fits_one_page(4096, 5));
        assert!(append_fits_one_page(4095, 1));
        assert!(!append_fits_one_page(4095, 2));
        assert!(append_fits_one_page(123, 0));
        let straddling: Vec<u64> = (0..5000u64)
            .filter(|n| !append_fits_one_page(13 + 4 * n, TAIL))
            .collect();
        assert_eq!(straddling, [1020, 2044, 3068, 4092]);
        assert!(straddling.iter().all(|n| (13 + 4 * n) % PAGE == 4093));
    }

    /// The claim makes the name appear with its content through a hard link of a flushed
    /// temporary file, leaves no temporary file, keeps a taken name's file, and a failed write
    /// creates nothing.
    #[test]
    fn a_claim_appears_whole_and_never_replaces_a_file() {
        let dir = scratch("claim");
        let sync = |f: &mut std::fs::File, b: &[u8]| f.write_all(b).and_then(|()| f.sync_all());
        assert_eq!(claim_new(&dir, "a.lmp", b"header+marker", sync), Ok(true));
        assert_eq!(std::fs::read(dir.join("a.lmp")).unwrap(), b"header+marker");
        assert_eq!(claim_new(&dir, "a.lmp", b"other", sync), Ok(false));
        assert_eq!(std::fs::read(dir.join("a.lmp")).unwrap(), b"header+marker");
        // the content is visible under the name the moment the name exists: the write sees no
        // file of that name yet
        let probe = dir.join("b.lmp");
        let r = claim_new(&dir, "b.lmp", b"xyz", |f, b| {
            assert!(!probe.exists(), "the name appears only after the write");
            f.write_all(b)
        });
        assert_eq!(r, Ok(true));
        let full = |f: &mut std::fs::File, b: &[u8]| {
            f.write_all(&b[..1])?;
            Err(std::io::Error::other("disk full"))
        };
        let r = claim_new(&dir, "c.lmp", b"demo", full);
        assert!(r.as_ref().is_err_and(|e| e.ends_with("disk full")), "{r:?}");
        assert_eq!(names(&dir), ["a.lmp", "b.lmp"], "no temporary file is left");
        // the hard link is what the claim used: the file has one link once the temporary is gone
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            assert_eq!(std::fs::metadata(dir.join("a.lmp")).unwrap().nlink(), 1);
        }
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// M5b's fallback claim: exclusive, keeps a taken name, writes the content.
    #[test]
    fn the_fallback_claim_is_exclusive() {
        let dir = scratch("fallback");
        let path = dir.join("a.lmp");
        assert_eq!(claim_create_new(&path, b"one"), Ok(true));
        assert_eq!(claim_create_new(&path, b"two"), Ok(false));
        assert_eq!(std::fs::read(&path).unwrap(), b"one");
        assert!(claim_create_new(&dir.join("no/such/dir.lmp"), b"x").is_err());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// An append overwrites the marker and extends the file by four bytes; it declines at a page
    /// boundary and when the file is not the length the app expects (both leave the file
    /// unchanged); a missing file is an error.
    #[test]
    fn an_append_overwrites_the_marker_or_declines_without_writing() {
        let dir = scratch("append");
        let mut demo = vec![110, 2, 1, 1, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0x80];
        std::fs::write(dir.join("a.lmp"), &demo).unwrap();
        assert_eq!(append(&dir, "a.lmp", 13, &[25, 0, 3, 1, 0x80]), Ok(true));
        demo.truncate(13);
        demo.extend([25, 0, 3, 1, 0x80]);
        assert_eq!(std::fs::read(dir.join("a.lmp")).unwrap(), demo);
        // the app's idea of the length is wrong: declined, nothing written
        assert_eq!(append(&dir, "a.lmp", 13, &[1, 2, 3, 4, 0x80]), Ok(false));
        assert_eq!(std::fs::read(dir.join("a.lmp")).unwrap(), demo);
        // a file of 1,020 tics: its marker at 4093, the append straddles 4096: declined
        let mut long = demo[..13].to_vec();
        long.extend(std::iter::repeat_n([0u8, 0, 0, 0], 1020).flatten());
        long.push(0x80);
        assert_eq!(long.len(), 4094);
        std::fs::write(dir.join("b.lmp"), &long).unwrap();
        assert_eq!(append(&dir, "b.lmp", 4093, &[1, 0, 0, 0, 0x80]), Ok(false));
        assert_eq!(std::fs::read(dir.join("b.lmp")).unwrap(), long);
        assert!(append(&dir, "missing.lmp", 13, &[0, 0, 0, 0, 0x80]).is_err());
        assert!(
            !dir.join("missing.lmp").exists(),
            "an append never creates a file"
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// A stand-in for the session file that records every `write` call (the offset and the bytes
    /// it took) and takes at most `limit` bytes per call.
    struct Calls {
        pos: u64,
        writes: Vec<(u64, Vec<u8>)>,
        limit: usize,
    }

    impl Write for Calls {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            let n = buf.len().min(self.limit);
            self.writes.push((self.pos, buf[..n].to_vec()));
            self.pos += n as u64;
            Ok(n)
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    impl Seek for Calls {
        fn seek(&mut self, to: SeekFrom) -> std::io::Result<u64> {
            match to {
                SeekFrom::Start(p) => {
                    self.pos = p;
                    Ok(p)
                }
                _ => Err(std::io::Error::other(
                    "the append seeks from the start only",
                )),
            }
        }
    }

    /// Review MAJ-1 (a): an append is exactly one `write` call, of the five bytes, at the old
    /// marker's offset; a call the system shortens is reported as it is and never followed by a
    /// second call (which a kill could separate from the first); a file of the wrong length gets
    /// no write at all.
    #[test]
    fn an_append_is_one_write_call_of_the_five_bytes_at_the_marker() {
        let tail = [25, 0, 3, 1, 0x80];
        let mut f = Calls {
            pos: 0,
            writes: Vec::new(),
            limit: usize::MAX,
        };
        assert_eq!(write_tail(&mut f, 14, 13, &tail).unwrap(), Some(5));
        assert_eq!(
            f.writes,
            [(13, tail.to_vec())],
            "one call of five bytes at 13"
        );
        let mut short = Calls {
            pos: 0,
            writes: Vec::new(),
            limit: 3,
        };
        assert_eq!(write_tail(&mut short, 4090, 4089, &tail).unwrap(), Some(3));
        assert_eq!(
            short.writes,
            [(4089, tail[..3].to_vec())],
            "a short write is not completed by a second call"
        );
        let mut wrong = Calls {
            pos: 0,
            writes: Vec::new(),
            limit: usize::MAX,
        };
        assert_eq!(write_tail(&mut wrong, 18, 13, &tail).unwrap(), None);
        assert!(wrong.writes.is_empty());
    }

    #[cfg(unix)]
    fn ino(path: &Path) -> u64 {
        use std::os::unix::fs::MetadataExt;
        std::fs::metadata(path).unwrap().ino()
    }

    /// Review MAJ-1 (b): a whole rewrite replaces the file (a new inode under the name) and never
    /// writes the old one: a second link to the old file still holds the old bytes afterwards, so
    /// the name held them until the rename.
    #[cfg(unix)]
    #[test]
    fn a_whole_rewrite_replaces_the_file_and_never_writes_the_old_one() {
        let dir = scratch("rewrite");
        let path = dir.join("a.lmp");
        let sync = |f: &mut std::fs::File, b: &[u8]| f.write_all(b).and_then(|()| f.sync_all());
        assert_eq!(claim_new(&dir, "a.lmp", b"old demo", sync), Ok(true));
        let old = ino(&path);
        std::fs::hard_link(&path, dir.join("old.link")).unwrap();
        save(&dir, "a.lmp", b"the new, longer demo").unwrap();
        assert_ne!(ino(&path), old, "the name now holds another file");
        assert_eq!(std::fs::read(&path).unwrap(), b"the new, longer demo");
        assert_eq!(ino(&dir.join("old.link")), old);
        assert_eq!(
            std::fs::read(dir.join("old.link")).unwrap(),
            b"old demo",
            "the old file was never written"
        );
        assert_eq!(
            names(&dir),
            ["a.lmp", "old.link"],
            "no temporary file is left"
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// Review MAJ-1 (c): the claimed name is the temporary file itself (the inode the write saw),
    /// with one link once the temporary name is gone. When the hard link fails, the fallback is
    /// taken, and says so ([`Claim::Created`]: a different file, with the content, and no
    /// temporary left); a link refused because the name exists meanwhile creates nothing.
    #[cfg(unix)]
    #[test]
    fn a_claimed_name_is_the_temporary_file_or_the_fallback_says_so() {
        use std::os::unix::fs::MetadataExt;
        let dir = scratch("claim-inode");
        // Each claim's temporary file is kept open (a second handle) until the end of the test,
        // so its inode stays allocated and cannot be handed to the next file created (ext4 reuses
        // a freed inode number at once, which made the fallback's comparison below flaky).
        let kept: std::cell::RefCell<Vec<std::fs::File>> = std::cell::RefCell::new(Vec::new());
        fn write<'a>(
            seen: &'a std::cell::Cell<u64>,
            kept: &'a std::cell::RefCell<Vec<std::fs::File>>,
        ) -> impl FnOnce(&mut std::fs::File, &[u8]) -> std::io::Result<()> + 'a {
            move |f, b| {
                seen.set(f.metadata()?.ino());
                kept.borrow_mut().push(f.try_clone()?);
                f.write_all(b).and_then(|()| f.sync_all())
            }
        }
        let link = |tmp: &Path, path: &Path| std::fs::hard_link(tmp, path);
        let remove = |tmp: &Path| std::fs::remove_file(tmp);
        let seen = std::cell::Cell::new(0);
        let linked = claim_with(
            &dir,
            "a.lmp",
            b"header+marker",
            write(&seen, &kept),
            link,
            remove,
        );
        assert_eq!(linked, Ok(Claim::Linked));
        assert_eq!(
            ino(&dir.join("a.lmp")),
            seen.get(),
            "the name is the temporary file"
        );
        assert_eq!(std::fs::metadata(dir.join("a.lmp")).unwrap().nlink(), 1);
        assert_eq!(std::fs::read(dir.join("a.lmp")).unwrap(), b"header+marker");
        // the link fails: the fallback, explicitly
        let tmp_seen = std::cell::Cell::new(0);
        let unsupported = |tmp: &Path, _: &Path| {
            assert!(tmp.exists(), "the temporary file exists at the link");
            Err(std::io::Error::from(std::io::ErrorKind::Unsupported))
        };
        let created = claim_with(
            &dir,
            "b.lmp",
            b"demo",
            write(&tmp_seen, &kept),
            unsupported,
            remove,
        );
        assert_eq!(created, Ok(Claim::Created));
        assert_ne!(
            ino(&dir.join("b.lmp")),
            tmp_seen.get(),
            "another file: the fallback"
        );
        assert_eq!(std::fs::read(dir.join("b.lmp")).unwrap(), b"demo");
        // the name is taken between the check and the link: nothing created
        let taken =
            |_: &Path, _: &Path| Err(std::io::Error::from(std::io::ErrorKind::AlreadyExists));
        let r = claim_with(&dir, "c.lmp", b"demo", write(&seen, &kept), taken, remove);
        assert_eq!(r, Ok(Claim::Taken));
        assert_eq!(names(&dir), ["a.lmp", "b.lmp"], "no temporary file is left");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// Second review M-1: the real [`claim_new`] makes the name by hard-linking the temporary file
    /// (the name's inode is the one the write saw, kept allocated by a second handle so that a
    /// fallback file could not reuse its number).
    #[cfg(unix)]
    #[test]
    fn the_real_claim_hard_links_the_temporary_file() {
        use std::os::unix::fs::MetadataExt;
        let dir = scratch("claim-real-link");
        let seen = std::cell::Cell::new(0);
        let mut kept = None;
        let write = |f: &mut std::fs::File, b: &[u8]| -> std::io::Result<()> {
            seen.set(f.metadata()?.ino());
            kept = Some(f.try_clone()?);
            f.write_all(b).and_then(|()| f.sync_all())
        };
        assert_eq!(claim_new(&dir, "a.lmp", b"header+marker", write), Ok(true));
        assert_eq!(
            ino(&dir.join("a.lmp")),
            seen.get(),
            "the name is the temporary file"
        );
        assert_eq!(std::fs::metadata(dir.join("a.lmp")).unwrap().nlink(), 1);
        drop(kept);
        assert_eq!(names(&dir), ["a.lmp"], "no temporary file is left");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// Review MIN-5: when the removal of the claim's temporary name fails, that name stays a
    /// second link to the session's file; a later whole rewrite must not write through it (it
    /// uses its own name), and a stale file at the rewrite's own temporary name (here a link to
    /// the session's file) is replaced, never written.
    #[cfg(unix)]
    #[test]
    fn a_failed_removal_of_the_claims_temporary_never_lets_a_rewrite_write_in_place() {
        let dir = scratch("claim-removal");
        assert_ne!(claim_temporary(&dir, "a.lmp"), temporary(&dir, "a.lmp"));
        let sync = |f: &mut std::fs::File, b: &[u8]| f.write_all(b).and_then(|()| f.sync_all());
        let link = |tmp: &Path, path: &Path| std::fs::hard_link(tmp, path);
        let refused = |_: &Path| Err(std::io::Error::other("sharing violation"));
        let r = claim_with(&dir, "a.lmp", b"header+marker", sync, link, refused);
        assert_eq!(r, Ok(Claim::Linked));
        let leftover = claim_temporary(&dir, "a.lmp");
        assert_eq!(
            ino(&leftover),
            ino(&dir.join("a.lmp")),
            "a second link remains"
        );
        save(&dir, "a.lmp", b"first rewrite").unwrap();
        assert_eq!(std::fs::read(dir.join("a.lmp")).unwrap(), b"first rewrite");
        assert_eq!(
            std::fs::read(&leftover).unwrap(),
            b"header+marker",
            "the claimed file was never written by the rewrite"
        );
        // a stale rewrite temporary that is a link to the session's file
        std::fs::hard_link(dir.join("a.lmp"), temporary(&dir, "a.lmp")).unwrap();
        std::fs::hard_link(dir.join("a.lmp"), dir.join("kept.link")).unwrap();
        save(&dir, "a.lmp", b"second rewrite").unwrap();
        assert_eq!(std::fs::read(dir.join("a.lmp")).unwrap(), b"second rewrite");
        assert_eq!(
            std::fs::read(dir.join("kept.link")).unwrap(),
            b"first rewrite",
            "the file at the stale temporary name was never written"
        );
        let mut want = vec!["a.lmp".to_string(), "kept.link".to_string()];
        want.push(leftover.file_name().unwrap().to_string_lossy().into_owned());
        want.sort();
        assert_eq!(names(&dir), want);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// Ruling 6, the match rule: only this module's own two temporary names, built from a
    /// session demo's name and a process id, match; anything near them does not.
    #[test]
    fn only_our_own_temporary_names_count_as_leftovers() {
        let dir = Path::new("d");
        for name in [
            "E1M1-skill2-20261120-123456.lmp",
            "E1M9-skill3-20261120-123456-9999.lmp",
        ] {
            for built in [temporary(dir, name), claim_temporary(dir, name)] {
                let built = built.file_name().unwrap().to_str().unwrap().to_string();
                assert!(is_leftover_name(&built), "{built}");
            }
        }
        for good in [
            "E1M1-skill2-20261120-123456.lmp.1.tmp",
            "E1M5-skill0-00000000-000000.lmp.4294967295.claim.tmp",
            "E1M1-skill2-20261120-123456-2.lmp.77.tmp",
        ] {
            assert!(is_leftover_name(good), "{good}");
        }
        for bad in [
            "E1M1-skill2-20261120-123456.lmp",
            "E1M1-skill2-20261120-123456.lmp.tmp",
            "E1M1-skill2-20261120-123456.lmp..tmp",
            "E1M1-skill2-20261120-123456.lmp.0.tmp",
            "E1M1-skill2-20261120-123456.lmp.012.tmp",
            "E1M1-skill2-20261120-123456.lmp.4294967296.tmp",
            "E1M1-skill2-20261120-123456.lmp.12a.tmp",
            "E1M1-skill2-20261120-123456.lmp.12.claim.tmp.tmp",
            "E1M1-skill2-20261120-123456.lmp.12.other.tmp",
            "E1M1-skill2-20261120-123456.lmp.12.TMP",
            "E1M1-skill2-20261120-123456.lmp.12.tmp~",
            "E1M0-skill2-20261120-123456.lmp.12.tmp",
            "E1M10-skill2-20261120-123456.lmp.12.tmp",
            "E2M1-skill2-20261120-123456.lmp.12.tmp",
            "E1M1-skill22-20261120-123456.lmp.12.tmp",
            "E1M1-skill2-2026112-123456.lmp.12.tmp",
            "E1M1-skill2-20261120-1234567.lmp.12.tmp",
            "E1M1-skill2-20261120-123456-1.lmp.12.tmp",
            "E1M1-skill2-20261120-123456-02.lmp.12.tmp",
            "E1M1-skill2-20261120-123456-10000.lmp.12.tmp",
            "E1M1-skill2-20261120-123456-2-3.lmp.12.tmp",
            "E1M1-skill2-20261120-123456.wad.12.tmp",
            "x.lmp.12.tmp",
            "notes.tmp",
            "a.lmp.12.tmp",
        ] {
            assert!(!is_leftover_name(bad), "{bad}");
        }
    }

    /// Ruling 6, the age rule: more than an hour by the modification time, with the clock
    /// injected; a time in the future is not old.
    #[test]
    fn only_files_more_than_an_hour_old_are_stale() {
        let now = std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1_000_000);
        let ago = |s: u64| now - std::time::Duration::from_secs(s);
        assert!(!is_stale(now, now));
        assert!(!is_stale(ago(3_600), now), "exactly an hour is not more");
        assert!(is_stale(ago(3_601), now));
        assert!(is_stale(ago(86_400), now));
        assert!(!is_stale(now + std::time::Duration::from_secs(5), now));
    }

    /// Ruling 6 on a directory: only the stale regular files of our two names are removed; a
    /// fresh one, a directory of such a name, a demo and a stale file of another name stay; a
    /// missing directory removes nothing.
    #[test]
    fn the_start_removes_only_our_stale_temporary_files() {
        let dir = scratch("leftovers");
        let now = std::time::SystemTime::now();
        let old = now - std::time::Duration::from_secs(2 * 3_600);
        let make = |name: &str, modified: std::time::SystemTime| {
            let f = std::fs::File::create(dir.join(name)).unwrap();
            f.set_modified(modified).unwrap();
        };
        make("E1M1-skill2-20261120-123456.lmp.41.tmp", old);
        make("E1M1-skill2-20261120-123456.lmp.41.claim.tmp", old);
        make("E1M1-skill2-20261120-123456.lmp.42.tmp", now);
        make("E1M1-skill2-20261120-123456.lmp", old);
        make("other.tmp", old);
        std::fs::create_dir(dir.join("E1M1-skill2-20261120-123456.lmp.43.tmp")).unwrap();
        let (removed, errors) = remove_leftovers(&dir, now);
        assert_eq!((removed, errors.len()), (2, 0), "{errors:?}");
        assert_eq!(
            names(&dir),
            [
                "E1M1-skill2-20261120-123456.lmp",
                "E1M1-skill2-20261120-123456.lmp.42.tmp",
                "E1M1-skill2-20261120-123456.lmp.43.tmp",
                "other.tmp"
            ]
        );
        assert_eq!(remove_leftovers(&dir.join("missing"), now), (0, vec![]));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// Review 3, P19: the clean-up removes regular files only. With `now` two hours ahead, every
    /// entry counts as old, so the age rule cannot hide the guard: a regular file of a leftover
    /// name goes; a directory and (on Unix) a symbolic link of leftover names stay, and so does the
    /// file the link points at.
    #[test]
    fn the_start_removes_regular_files_only_whatever_their_age() {
        let dir = scratch("leftover-kinds");
        let later = std::time::SystemTime::now() + std::time::Duration::from_secs(2 * 3_600);
        std::fs::File::create(dir.join("E1M1-skill2-20261120-123456.lmp.41.tmp")).unwrap();
        std::fs::create_dir(dir.join("E1M1-skill2-20261120-123456.lmp.43.tmp")).unwrap();
        // off Unix nothing is added below
        #[cfg_attr(not(unix), allow(unused_mut))]
        let mut want = vec!["E1M1-skill2-20261120-123456.lmp.43.tmp".to_string()];
        #[cfg(unix)]
        {
            std::fs::File::create(dir.join("target")).unwrap();
            std::os::unix::fs::symlink(
                dir.join("target"),
                dir.join("E1M1-skill2-20261120-123456.lmp.44.claim.tmp"),
            )
            .unwrap();
            want.insert(
                1,
                "E1M1-skill2-20261120-123456.lmp.44.claim.tmp".to_string(),
            );
            want.push("target".to_string());
        }
        let (removed, errors) = remove_leftovers(&dir, later);
        assert_eq!((removed, errors.len()), (1, 0), "{errors:?}");
        assert_eq!(names(&dir), want);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// Several tics through one open: each its own write at the marker the previous one left, the
    /// result equal to one append at a time; a page straddle anywhere declines before writing.
    #[test]
    fn several_tics_append_like_single_appends_or_decline_whole() {
        let dir = scratch("append-tics");
        let mut demo = vec![110, 2, 1, 1, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0x80];
        std::fs::write(dir.join("a.lmp"), &demo).unwrap();
        let tails = [[1, 0, 0, 0, 0x80], [2, 0, 0, 0, 0x80], [3, 0, 0, 0, 0x80]];
        assert_eq!(append_tics(&dir, "a.lmp", 13, &tails), Ok(true));
        demo.truncate(13);
        for t in &tails {
            demo.extend(&t[..4]);
        }
        demo.push(0x80);
        assert_eq!(std::fs::read(dir.join("a.lmp")).unwrap(), demo);
        // the second of two would straddle 4096: declined, nothing written
        let mut long = demo[..13].to_vec();
        long.extend(std::iter::repeat_n([0u8, 0, 0, 0], 1019).flatten());
        long.push(0x80);
        std::fs::write(dir.join("b.lmp"), &long).unwrap();
        assert_eq!(append_tics(&dir, "b.lmp", 4089, &tails[..2]), Ok(false));
        assert_eq!(std::fs::read(dir.join("b.lmp")).unwrap(), long);
        assert_eq!(append_tics(&dir, "b.lmp", 4089, &tails[..1]), Ok(true));
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
