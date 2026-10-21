use std::io;

fn main() {
    let cwd = match std::env::current_dir() {
        Ok(cwd) => cwd,
        Err(error) => {
            eprintln!("gdfmt: cannot read current directory: {error}");
            std::process::exit(2);
        }
    };
    let code = gdfmt_cli::run(
        std::env::args_os(),
        &cwd,
        &mut io::stdin().lock(),
        &mut io::stdout().lock(),
        &mut io::stderr().lock(),
    );
    std::process::exit(code);
}
