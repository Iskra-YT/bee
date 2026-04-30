mod cli;
mod run;
mod init;
mod parser;
mod yaml;

use clap::Parser;
use cli::Cli;

fn main() {
    let cli = Cli::parse();

    match cli.command {
        cli::Commands::Run { tag, task } => {
            run::run_main(tag, task);
        },

        cli::Commands::Init => {
            init::run_init().unwrap();
            println!("Initialization complete!");
        }
    }
}
