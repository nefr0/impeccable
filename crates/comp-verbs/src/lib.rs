//! impeccable-comp-verbs: the comp-fidelity verb orchestrators
//! (`comp-spec`, `comp-diff`, `build-phase`), ported from the skill's JS
//! scripts of the same name.
//!
//! OPEN and free of the closed `core` crate. It wires only the pure `comp`
//! foundation plus `common` (`Io`). The one thing it cannot do on its own is
//! injected by the CLI so `core` stays out: build-phase's organic-clip-path CSS
//! scan (a rule that lives in `core`), a [`build_phase::OrganicScan`] closure.

pub mod approved_comp;
pub mod build_phase;
pub mod completion;
pub mod comp_diff;
pub mod comp_spec;
pub mod displacement;
mod map_inspection;
mod util;

use impeccable_common::Io;

/// `impeccable comp-spec ...`
pub fn run_comp_spec(argv: &[String], io: &mut Io) -> i32 {
    comp_spec::run(argv, io)
}

/// `impeccable comp-diff ...`
pub fn run_comp_diff(argv: &[String], io: &mut Io) -> i32 {
    comp_diff::run(argv, io)
}

/// `impeccable build-phase ...`. `organic_scan` is the injected CSS
/// organic-clip-path scanner (pass [`build_phase::no_organic_scan`] to skip it,
/// matching the JS degraded path).
pub fn run_build_phase(argv: &[String], io: &mut Io, organic_scan: build_phase::OrganicScan) -> i32 {
    build_phase::run(argv, io, organic_scan)
}

pub mod asset_capture;

pub mod entry_capture;
