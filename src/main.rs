use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(help = "The source directory.")]
    source: String,
    #[arg(short, long, default_value = ".", help = "The destination directory")]
    dest: String,
}

fn main() {
    // Initialize Logging.
    let log_environ = env_logger::Env::new()
        .filter("IMPERTIO_LOG")
        .write_style("IMPERTIO_LOG_STYLE");
    let mut log_builder = env_logger::Builder::new();

    log_builder.filter_level(log::LevelFilter::Info);
    log_builder.parse_env(log_environ);
    log_builder.init();

    // Parse Arguments.
    let args = Args::parse();

    log::info!("Beginning to process `{}`", args.source);
    log::info!("Outputting to `{}`", args.dest);

    impertio::files::FileHandler::new(&args.source).handle_files(args.dest, args.source);

    log::info!("Done.");
}
