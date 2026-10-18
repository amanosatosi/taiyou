//! CLI package information, also available to workspace validation tools.

/// Package version reported by `taiyou-cli --version`.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(feature = "native")]
pub mod render;
pub fn main_entry() -> std::process::ExitCode {
    use std::{ffi::OsStr, process::ExitCode};
    let mut args = std::env::args_os().skip(1);
    let command = args.next();
    if command.as_deref() == Some(OsStr::new("--version")) && args.len() == 0 {
        println!("taiyou {VERSION}");
        return ExitCode::SUCCESS;
    }
    #[cfg(feature = "native")]
    {
        if command.as_deref() == Some(OsStr::new("render")) {
            match render::run(args.collect()) {
                Ok(()) => return ExitCode::SUCCESS,
                Err(e) => eprintln!("taiyou: {e}; event={:?} span={:?}", e.event(), e.span()),
            }
        } else {
            eprintln!("taiyou: InvalidArgument: expected render or --version");
        }
    }
    #[cfg(not(feature = "native"))]
    eprintln!(
        "taiyou: UnsupportedFeature: rendering requires the native feature (no output produced)"
    );
    ExitCode::from(2)
}
