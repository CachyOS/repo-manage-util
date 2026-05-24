use std::env;

use clap::CommandFactory;
use clap_complete::{generate_to, Shell};

include!("src/args.rs");

fn main() {
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());

    let mut command = Cli::command();
    for shell in [Shell::Bash, Shell::Fish, Shell::Zsh] {
        generate_to(shell, &mut command, "repo-manage-util", &out_path).unwrap_or_else(|err| {
            panic!(
                "Couldn't generate completion for shell {:?} in {}: {}",
                shell,
                out_path.display(),
                err
            )
        });
    }
}
