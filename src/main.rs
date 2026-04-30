use clap::Parser;

mod cli;
mod file;
mod yaml;
mod run;
mod init;

fn main() {
    let cli = cli::Cli::parse();

    match cli.command {
        cli::Commands::Run { tag, task } => {
            run::run_main(tag, task);
        },

        cli::Commands::Init => {
            init::run_init().unwrap();
        }
    }
}
