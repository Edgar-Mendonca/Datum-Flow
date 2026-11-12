//! `flow-cli`: Datum Flow from the command line (see `flow-cli help`).

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    // An internal error must not look like a result: report it and exit with 2.
    let (code, text) = std::panic::catch_unwind(|| flow_cli::cli::run(&args)).unwrap_or_else(|_| (2, "Datum Flow met an internal error. Please report it: https://github.com/Edgar-Mendonca/Datum-Flow/issues\n".into()));
    if code == 2 { eprint!("{text}") } else { print!("{text}") }
    std::process::exit(code);
}
