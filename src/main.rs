// SPDX-FileCopyrightText: 2024 Ohin "Kazani" Taylor <kazani@kazani.dev>
// SPDX-License-Identifier: MIT

use std::{path::PathBuf, str::FromStr};

use clap::Parser;
use impertio::config::Config;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(help = "The source directory.")]
    source: String,
    #[arg(short, long, default_value = ".", help = "The destination directory")]
    dest: String,
}

fn main() -> anyhow::Result<()>{
    let log_environ = env_logger::Env::new()
        .filter("IMPERTIO_LOG")
        .write_style("IMPERTIO_LOG_STYLE");
    let mut log_builder = env_logger::Builder::new();

    log_builder.filter_level(log::LevelFilter::Info);
    log_builder.parse_env(log_environ);
    log_builder.init();

    let args = Args::parse();

    let mut config_path = PathBuf::from_str(&args.source)?;
    config_path.push("impertio.yaml");

    let config: Config = serde_yaml::from_str(&std::fs::read_to_string(config_path)?)?;

    log::info!("Beginning to process `{}`", args.source);
    log::info!("Outputting to `{}`", args.dest);

    let mut fd = impertio::files::FileDispatcher::new(&args.source, config);
    
    fd.handle_files(args.dest, args.source)?;

    log::info!("Done.");

    Ok(())
}
