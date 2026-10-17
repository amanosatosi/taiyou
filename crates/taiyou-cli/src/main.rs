use std::ffi::OsStr;
use std::process::ExitCode;

fn main() -> ExitCode {
    let mut args = std::env::args_os().skip(1);
    if args.next().as_deref() == Some(OsStr::new("--version")) && args.next().is_none() {
        println!("taiyou {}", taiyou_cli::VERSION);
        return ExitCode::SUCCESS;
    }

    eprintln!("taiyou: rendering is not implemented (T001 foundation; no output produced)");
    ExitCode::from(2)
}
