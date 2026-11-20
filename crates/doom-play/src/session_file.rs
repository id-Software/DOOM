//! The session demo on disk (M5d): created only with its content, kept current tic by tic, and
//! meant to be a valid demo at every instant, so a session killed at any point (Ctrl+C, a closed
//! console window, a lost display connection, `SIGKILL`) leaves a file that replays every tic it
//! ran, or every tic but the last. The argument below covers local file systems whose buffered
//! writes go through the page cache (ext4, tmpfs on Linux); it does not cover FUSE, NFS or 9p
//! (WSL's `/mnt/c`), the fallback claim's window on file systems without hard links, or a write
//! that keeps failing (the file then stays at its last good state and falls behind).
//!
//! - **Creation** ([`claim_new`]): the first content (the 13-byte header and the end marker) is
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
//!   1,024 tics): there [`append_fits_one_page`] says no and [`append`] declines, and the app
//!   rewrites the file whole through [`save`] (a synced temporary and a rename, atomic by the
//!   rename). The one remaining way the kernel loop can stop inside a folio is a short copy from
//!   the caller's buffer (a fault on the five source bytes, which the app has just written and
//!   which are resident): not expected.
//! - **What the tests show.** Deterministic unit tests pin what the code does: one `write` call
//!   of the five bytes per append ([`write_tail`]), a whole rewrite that replaces the file (a new
//!   file renamed over it, never the old one written), and a claim whose name appears as the
//!   temporary file (the hard link) or, where the link fails, through the fallback. The kill test
//!   (`kill_tests`) and `tools/reference/m5d_kill_check.py` are evidence for the steady state: no
//!   gross fault (a file that falls behind, loses its marker or replays differently) in the
//!   kills they make. The kill test's self-test shows that it catches a torn append whose two
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
//! The file is opened for each append and closed after it (no handle is kept): 35 opens a second
//! cost nothing next to a tic, no handle can be left open across the rename of a whole rewrite
//! (Windows refuses to rename over an open file), and [`crate::FsHost`] keeps its fields.

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
/// ([`claim_create_new`]).
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

/// [`claim_new`] with the hard link (`link(tmp, path)`) and the removal of the temporary name
/// after it (`remove_tmp(tmp)`) passed in, so that tests can make either fail.
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

/// M5b's claim, the fallback of [`claim_new`]: creates `path` exclusively (`create_new`) and
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
    let path = dir.join(name);
    let tmp = temporary(dir, name);
    let written = fresh(&tmp)
        .and_then(|mut f| f.write_all(bytes).and_then(|()| f.sync_all()))
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
    if !append_fits_one_page(at, TAIL) {
        return Ok(false);
    }
    let path = dir.join(name);
    let fail = |e: std::io::Error| format!("{}: {e}", path.display());
    let mut f = std::fs::OpenOptions::new()
        .write(true)
        .open(&path)
        .map_err(fail)?;
    let len = f.metadata().map_err(fail)?.len();
    match write_tail(&mut f, len, at, tail).map_err(fail)? {
        None => Ok(false),
        Some(5) => Ok(true),
        Some(n) => Err(format!(
            "{}: {n} of 5 bytes written; the file is rewritten whole next",
            path.display()
        )),
    }
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
}
