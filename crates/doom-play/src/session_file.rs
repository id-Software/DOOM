//! The session demo on disk (M5d): created only with its content, kept current tic by tic, and a
//! valid demo at every instant, so a session killed at any point (Ctrl+C, a closed console
//! window, a lost display connection, `SIGKILL`) leaves a file that replays every tic it ran, or
//! every tic but the last.
//!
//! - **Creation** ([`claim_new`]): the first content (the 13-byte header and the end marker) is
//!   written to a temporary file, flushed, and hard-linked to the session's name, which fails if
//!   the name exists (so no file is ever replaced); then the temporary name is removed. The name
//!   therefore never appears empty or partial. A kill between the link and the removal leaves a
//!   `*.tmp` file beside the demo (never a `.lmp`). Where the file system refuses the hard link
//!   (FAT, exFAT, some network shares), the claim falls back to M5b's exclusive create
//!   (`create_new`) and a write, which leaves one window: a kill between the create and the write
//!   leaves an empty `.lmp`.
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
//! - **Why one five-byte write lands all or nothing under `SIGKILL` on Linux** (this rests on the
//!   kernel's behaviour, not on any promise of POSIX; the evidence is the kill test,
//!   `kill_tests`, and `tools/reference/m5d_kill_check.py`): a buffered write to a regular file
//!   on ext4 or tmpfs goes through `generic_perform_write` (Linux v6.18 mm/filemap.c:4227-4318;
//!   ext4 fs/ext4/file.c:299, tmpfs mm/shmem.c:3466), which copies the data one page-cache folio
//!   at a time and checks for a pending fatal signal only at the top of each folio's iteration
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
//!   which are resident): not expected, and the kill test is the evidence that it does not occur
//!   in practice.
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

/// The temporary name beside `name` for this process (two processes never share it).
fn temporary(dir: &Path, name: &str) -> PathBuf {
    dir.join(format!("{name}.{}.tmp", std::process::id()))
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
    let path = dir.join(name);
    let fail = |e: std::io::Error| format!("{}: {e}", path.display());
    std::fs::create_dir_all(dir).map_err(fail)?;
    // a name already taken is left alone before anything is written (the link below checks it
    // again, atomically, for a name taken meanwhile)
    if std::fs::symlink_metadata(&path).is_ok() {
        return Ok(false);
    }
    let tmp = temporary(dir, name);
    let written = std::fs::File::create(&tmp).and_then(|mut f| write(&mut f, bytes));
    if let Err(e) = written {
        let _ = std::fs::remove_file(&tmp);
        return Err(fail(e));
    }
    let linked = std::fs::hard_link(&tmp, &path);
    let _ = std::fs::remove_file(&tmp);
    match linked {
        Ok(()) => Ok(true),
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => Ok(false),
        // no hard links here: M5b's exclusive create, with its window (see the module
        // documentation); the content is written again from `bytes`
        Err(_) => claim_create_new(&path, bytes),
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

/// Rewrites `dir/name` whole: writes a temporary file, flushes it to the disk and renames it over
/// the session's file (atomic: the name holds the old or the new content); a failed write removes
/// the temporary file.
pub(crate) fn save(dir: &Path, name: &str, bytes: &[u8]) -> Result<(), String> {
    let path = dir.join(name);
    let tmp = temporary(dir, name);
    let written = std::fs::File::create(&tmp)
        .and_then(|mut f| f.write_all(bytes).and_then(|()| f.sync_all()))
        .and_then(|()| std::fs::rename(&tmp, &path));
    if written.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    written.map_err(|e| format!("{}: {e}", path.display()))
}

/// Extends `dir/name` by one tic: writes the five bytes of `tail` at byte `at` in one `write`
/// (see the module documentation). Declines (`Ok(false)`, and the app rewrites the file whole)
/// when the five bytes would straddle a [`PAGE`] boundary, or when the file does not hold exactly
/// `at + 1` bytes (it is not the file the app last wrote; the whole rewrite repairs it). A write
/// that the system shortens is an error: the file may then lack its marker, and the app's next
/// write is the whole rewrite.
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
    if f.metadata().map_err(fail)?.len() != at + 1 {
        return Ok(false);
    }
    f.seek(SeekFrom::Start(at)).map_err(fail)?;
    match f.write(tail).map_err(fail)? {
        5 => Ok(true),
        n => Err(format!(
            "{}: {n} of 5 bytes written; the file is rewritten whole next",
            path.display()
        )),
    }
}
