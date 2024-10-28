# repo-manage-util

A command-line utility for managing Arch Linux repositories.

## Features

- **Reset:** Resets the repository database and removes outdated packages.
- **Update:** Updates the repository database with new packages and removes stale packages.
- **MovePkgsToRepo:** Moves packages from the current directory to the repository.
- **MovePkgs:** Moves packages from one repository to another repository.
- **IsPkgsUpToDate:** Checks if the packages in the repository are up-to-date.
- **CleanupBackupDir:** Cleans up the backup directory, removing older package versions.

## Installation

### From the AUR

You can install `repo-manage-util` directly from the AUR using your preferred AUR helper. For example, using `paru`:

TBD

### Building from Source

Or can be built from source:

```bash
git clone https://github.com/cachyos/repo-manage-util.git
cd repo-manage-util
cargo install --path .
```

This will build and install the binary to `$HOME/.cargo/bin/`. Make sure this directory is in your `PATH` environment variable.

## Configuration

The configuration file is located at `~/.config/repo-manage/config.toml` or `/etc/repo-manage/config.toml`.

**Example Configuration:**

```toml
[profiles.myrepo]
repo = "/path/to/myrepo.db.tar.zst"
add_params = ["--sign", "--include-sigs"]
rm_params = ["--sign"]
require_signature = true
backup = true
backup_dir = "/path/to/backup/dir"
backup_num = 3  # Keep last 3 versions
debug_dir = "/path/to/debug/dir"
interactive = false
```

**Explanation of the Example Configuration:**

- **`[profiles.myrepo]`**: This defines a profile named "myrepo". You can have multiple profiles for different repositories.
- **`repo = "/path/to/myrepo.db.tar.zst"`**: This specifies the path to the repository database file (the `.db.tar.zst` file).
- **`add_params = ["--sign", "--include-sigs"]`**: These are additional parameters that will be passed to the `repo-add` command when adding packages to the repository. In this case, it's telling `repo-add` to sign the database and include signatures.
- **`rm_params = ["--sign"]`**: Similar to `add_params`, these are additional parameters passed to the `repo-remove` command, used when removing packages from the repository. Here, it tells `repo-remove` to sign the database after removal.
- **`require_signature = true`**: This setting enforces that packages must have valid signatures before being added to the repository. This is a good security practice.
- **`backup = true`**: This enables the backup feature. When enabled, outdated packages will be moved to the backup directory instead of being deleted.
- **`backup_dir = "/path/to/backup/dir"`**: This specifies the directory where outdated packages will be backed up.
- **`backup_num = 3`**: This sets a limit on the number of versions to keep for each package in the backup directory. In this case, only the last 3 versions of each package will be kept.
- **`debug_dir = "/path/to/debug/dir"`**: This option is used to specify a directory where debug packages should be stored.
- **`interactive = false`**: This disables interactive mode. When disabled, the tool will not prompt for confirmation before performing actions.

**Configuration Options:**

- **repo:** Path to the repository database file.
- **add_params:** Additional parameters to pass to `repo-add`.
- **rm_params:** Additional parameters to pass to `repo-remove`.
- **require_signature:** Whether to require package signatures.
- **backup:** Whether to backup outdated packages.
- **backup_dir:** Directory to store backup packages.
- **backup_num:** Number of package versions to keep in the backup directory.
- **debug_dir:** Directory to store debug packages.
- **interactive:** Whether to prompt for confirmation before performing actions.

## Usage

```
repo-manage-util --profile <PROFILE> [COMMAND]
```

**Available Commands:**

- **reset:** Resets the repository.
- **update:** Updates the repository.
- **move-pkgs-to-repo:** Moves packages from the current directory to the repository.
- **move-pkgs** Moves packages from one repository to another repository.
- **is-pkgs-up-to-date:** Checks if the packages in the repository are up-to-date.
- **cleanup-backup-dir:** Cleans up the backup directory.

**Example:**

```bash
repo-manage-util --profile myrepo update
```

For comprehensive usage instructions and examples, please refer to the **Usage** section in the main documentation (available after installation using `repo-manage-util --help`).

## Contributing

Contributions are welcome! Please open an issue or submit a pull request on GitHub.

## License

This project is licensed under the GPLv3 License. See the LICENSE file for details.
