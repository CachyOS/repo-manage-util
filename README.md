# repo-manage-util

A command-line utility for managing Arch Linux repositories.

## Features

- **Reset:** Resets the repository database and removes outdated packages.
- **Update:** Updates the repository database with new packages and removes stale packages.
- **Sync:** Updates the repository database with newer packages from the reference repository database.
- **MovePkgsToRepo:** Moves packages from the current directory to the repository.
- **MovePkgs:** Moves packages from one repository to another repository.
- **IsPkgsUpToDate:** Checks if the packages in the repository are up-to-date.
- **CleanupBackupDir:** Cleans up the backup directory, removing older package versions.

## Installation

### From the AUR

TBD: You can install `repo-manage-util` directly from the AUR using your preferred AUR helper.

### Building from Source

```bash
git clone https://github.com/cachyos/repo-manage-util.git
cd repo-manage-util
cargo install --path .
```

This will build and install the binary to `$HOME/.cargo/bin/`. Make sure this directory is in your `PATH` environment variable.

## Configuration and Deployment

For detailed information on configuration and deployment of the PostgreSQL integration and API service, please see the [Configuration and PostgreSQL Integration Guide](/CONFIG_AND_PG_README.md).

## Usage

```bash
repo-manage-util --profile <PROFILE> [COMMAND]
```

**Available Commands:**

| Command             | Description                                                              |
| ------------------- | ------------------------------------------------------------------------ |
| `reset`             | Resets the repository.                                                   |
| `update`            | Updates the repository.                                                  |
| `sync`              | Syncs repository with the reference repository.                          |
| `move-pkgs-to-repo` | Moves packages from the current directory to the repository.             |
| `move-pkgs`         | Moves packages from one repository to another repository.                |
| `is-pkgs-up-to-date`| Checks if the packages in the repository are up-to-date.                 |
| `cleanup-backup-dir`| Cleans up the backup directory.                                          |

**Example:**

```bash
# Update the repository defined in the 'myrepo' profile
repo-manage-util --profile myrepo update
```

For comprehensive usage instructions and examples, please refer to the **Usage** section in the main documentation (available after installation using `repo-manage-util --help`).

## Contributing

Contributions are welcome! Please open an issue or submit a pull request on GitHub.

## License

This project is licensed under the GPLv3 License. See the LICENSE file for details.
