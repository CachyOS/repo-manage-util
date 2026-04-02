use std::path::PathBuf;

use clap::Parser;

#[derive(Parser, PartialEq, Debug)]
#[command(author, version, about, long_about = None)]
#[clap(subcommand_negates_reqs = true)]
pub struct Cli {
    /// Profile to use from the configuration file
    #[arg(global = true, short, long)]
    profile: Option<String>,

    /// Profile to use from the configuration file
    #[arg(global = true, short, long)]
    from: Option<String>,
    /// Profile to use from the configuration file
    #[arg(global = true, short, long)]
    to: Option<String>,

    /// Flag to update only postgres DB for Reset command
    #[arg(global = true, short, long)]
    only_pg: bool,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Parser, PartialEq, Debug)]
pub struct SingleProfileCli {
    /// Profile to use from the configuration file
    #[clap(from_global)]
    pub profile: String,

    /// Flag to update only postgres DB for Reset command
    #[clap(from_global)]
    pub only_pg: bool,
}

#[derive(Parser, PartialEq, Debug)]
pub struct FromToProfileCli {
    /// Profile to use from the configuration file (for move-pkgs) FROM repo
    #[clap(from_global)]
    pub from: String,
    /// Profile to use from the configuration file (for move-pkgs) TO repo
    #[clap(from_global)]
    pub to: String,
}

#[derive(Parser, PartialEq, Debug)]
pub struct ProfileAurCli {
    /// Profile to use from the configuration file
    #[clap(from_global)]
    pub profile: String,

    /// Build order based on dependency tree into a file
    #[arg(short, long, value_name = "FILE")]
    pub order_file: Option<PathBuf>,

    /// Dry run (without downloading sources)
    #[arg(short = 'n', long)]
    pub dry_run: bool,
}

#[derive(Parser, PartialEq, Debug)]
pub struct CheckRebuildCli {
    /// Profile to use from the configuration file
    #[clap(from_global)]
    pub profile: String,

    /// Path to pacman.conf for resolving .so dependencies (defaults to /etc/pacman.conf)
    #[arg(long, value_name = "PATH")]
    pub pacman_conf: Option<PathBuf>,
}

#[derive(Parser, PartialEq, Debug)]
pub enum Commands {
    /// Reset the repository
    Reset(SingleProfileCli),
    /// Update the repository
    Update(SingleProfileCli),
    /// Sync repository with the reference repository
    Sync(SingleProfileCli),
    /// Moves packages from current directory into the repository
    MovePkgsToRepo(SingleProfileCli),
    /// Moves packages from one repository to another repository
    MovePkgs(FromToProfileCli),
    /// Check if the packages are up-to-date
    IsPkgsUpToDate(SingleProfileCli),
    /// Cleans up the backup directory,
    /// removing the N amount of packages if configured to do so
    CleanupBackupDir(SingleProfileCli),
    // Check if we have only certain amount of debug packages in the debug repository
    // IsDebugPkgsOk, // ok maybe not implemented
    /// Pulls outdated packages from AUR with generated build order
    Aur(ProfileAurCli),
    /// Check packages for broken .so dependencies (packages needing rebuild)
    CheckRebuild(CheckRebuildCli),
}

#[cfg(test)]
mod tests {
    use crate::args::{Cli, Commands, FromToProfileCli, SingleProfileCli};

    use clap::Parser;

    #[test]
    fn single_profile_check() {
        assert_eq!(Cli::parse_from(["test", "reset", "--profile", "abcd"]), Cli {
            profile: Some("abcd".to_owned()),
            to: None,
            from: None,
            only_pg: false,
            command: Commands::Reset(SingleProfileCli {
                profile: "abcd".to_owned(),
                only_pg: false
            })
        });

        assert_eq!(Cli::parse_from(["test", "--profile", "abcd", "reset"]), Cli {
            profile: Some("abcd".to_owned()),
            to: None,
            from: None,
            only_pg: false,
            command: Commands::Reset(SingleProfileCli {
                profile: "abcd".to_owned(),
                only_pg: false
            })
        });
    }

    #[test]
    fn from_to_profile_check() {
        assert_eq!(Cli::parse_from(["test", "move-pkgs", "--to", "abcd", "--from", "dcba"]), Cli {
            profile: None,
            to: Some("abcd".to_owned()),
            from: Some("dcba".to_owned()),
            only_pg: false,
            command: Commands::MovePkgs(FromToProfileCli {
                to: "abcd".to_owned(),
                from: "dcba".to_owned()
            })
        });

        assert_eq!(Cli::parse_from(["test", "--to", "abcd", "--from", "dcba", "move-pkgs"]), Cli {
            profile: None,
            to: Some("abcd".to_owned()),
            from: Some("dcba".to_owned()),
            only_pg: false,
            command: Commands::MovePkgs(FromToProfileCli {
                to: "abcd".to_owned(),
                from: "dcba".to_owned()
            })
        });
    }
}
