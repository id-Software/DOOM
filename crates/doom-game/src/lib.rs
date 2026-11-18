//! The platform-free app core of the playable port, milestone M5b (decision S19 of
//! `docs/M5B_REFERENCE_INVESTIGATION.md` §4; design in §3.4). The original id Software Linux
//! DOOM 1.10 source is the behavioural authority; every `file:line` citation below is into
//! `linuxdoom-1.10/` at `a77dfb96`.
//!
//! This crate has no windowing, audio, thread or wall-clock dependency, and no `unsafe`
//! (forbidden by the workspace lints). It takes DOOM key codes, never platform key codes: the
//! binary crate translates the window library's keys.
//!
//! - [`input`]: the genuine event-to-command path (`G_Responder`'s game part, `G_BuildTiccmd`
//!   and the demo write and re-read of `G_WriteDemoTiccmd`) and the app's event layer in front
//!   of it (decisions S4–S7);
//! - [`app`]: the session, the 35 Hz loop step on a clock the platform supplies, the session
//!   recorder, the app's pause and its own end and refusal screens, and restart (decisions S3,
//!   S8–S12, S21). Time and files come in from the platform through [`app::Host`] and
//!   [`app::App::update`]'s argument.

pub mod app;
pub mod input;
