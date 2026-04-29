use clap::Parser;

mod cli;
mod file;
mod yaml;
mod run;

fn main() {
    let cli = cli::Cli::parse();

    match cli.command {
        cli::Commands::Run { tag, no_cache, only } => {
            run::run_main(tag, no_cache, only);
        }
    }
}
