fn main() {
    let (cli, locale) = match diskscry::cli::parse() {
        Ok(parsed) => parsed,
        Err(error) => error.exit(),
    };
    if let Err(error) = diskscry::cli::run(cli, locale) {
        eprintln!(
            "{}: {}",
            locale.messages().error,
            locale.format_error(error.as_ref())
        );
        std::process::exit(1);
    }
}
