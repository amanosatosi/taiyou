//! Frozen external corpus adapter. No candidate-supplied PASS records.
use std::{ffi::OsString, path::Path, process::ExitCode};

pub fn run(root: &Path, args: &[OsString]) -> ExitCode {
    crate::report::launch(root, "compare.py", args)
}
