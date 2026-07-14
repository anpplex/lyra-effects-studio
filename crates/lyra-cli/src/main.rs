use std::process::ExitCode;

fn main() -> ExitCode {
    if std::env::args().nth(1).as_deref() == Some("--version") {
        println!("lyra-effects {}", env!("CARGO_PKG_VERSION"));
        ExitCode::SUCCESS
    } else {
        eprintln!("usage: lyra-effects --version");
        ExitCode::from(2)
    }
}
