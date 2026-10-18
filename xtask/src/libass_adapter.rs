//! Separate pinned native oracle; never substitutes Taiyou classifications.
use std::{ffi::OsString, path::Path, process::ExitCode};

pub fn run(root: &Path, args: &[OsString]) -> ExitCode {
    crate::report::launch(root, "libass_adapter.py", args)
}
