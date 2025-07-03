# repo-manage-util: Configuration and PostgreSQL Integration Guide

This document provides a comprehensive guide to configuring the `repo-manage-util` and setting up its PostgreSQL integration for advanced package management and analytics.

## Table of Contents

1.  [**Configuration**](#1-configuration)
    *   [Configuration File Location](#configuration-file-location)
    *   [Global Settings](#global-settings)
    *   [Profile-Based Repository Management](#profile-based-repository-management)
    *   [Detailed Profile Parameters](#detailed-profile-parameters)
2.  [**PostgreSQL Integration**](#2-postgresql-integration)
    *   [Overview](#overview)
    *   [Schema](#schema)
3.  [**Deployment with Docker**](#3-deployment-with-docker)
    *   [Prerequisites](#prerequisites)
    *   [Setup and Deployment Steps](#setup-and-deployment-steps)
    *   [API Service](#api-service)

---

## 1. Configuration

The utility is configured using a TOML file. For a detailed, commented example, please see `example-config.toml`.

### Configuration File Location

By default, the utility looks for a configuration file in the following locations:
1.  `~/.config/repo-manage-util/config.toml`
2.  `/etc/repo-manage-util/config.toml`

You can also specify a custom path using the `--config` command-line argument.

### Global Settings

This setting is placed at the root of the configuration file.

*   `postgresql_url` (Optional, String): Enables integration with a PostgreSQL database. The URL must be in the format `postgresql://<user>:<password>@<host>:<port>/<database_name>`. This is required for the API service and PostgreSQL repo logging.

### Profile-Based Repository Management

The `[profiles]` section allows you to manage multiple, distinct repositories from a single configuration. Each repository is defined under a unique profile name (e.g., `[profiles.my-stable-repo]`).

### Detailed Profile Parameters

These parameters are nested under a profile (e.g., `[profiles.my-repo]`).

| Parameter           | Type      | Required | Default                               | Description                                                                                                                                                           |
| ------------------- | --------- | -------- | ------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `repo`              | String    | **Yes**  | -                                     | Absolute path to the repository's database file (e.g., `/srv/repo/myrepo.db.tar.zst`). Package files are assumed to be in the same directory.                     |
| `add_params`        | [String]  | No       | `["--sign", "--include-sigs", "--verify"]` | A list of parameters passed directly to the `repo-add` command.                                                                                                       |
| `rm_params`         | [String]  | No       | `["--sign"]`                          | A list of parameters passed directly to the `repo-remove` command.                                                                                                    |
| `require_signature` | Boolean   | No       | `true`                                | If `true`, prevents adding packages that do not have a corresponding `.sig` signature file.                                                                           |
| `backup`            | Boolean   | No       | `false`                               | If `true`, older package files are moved to `backup_dir` when updated. If `false`, they are deleted.                                                                  |
| `backup_dir`        | String    | No       | -                                     | The directory for storing backed-up packages. Can be an absolute or relative path.                                                                                    |
| `backup_num`        | Integer   | No       | -                                     | The number of old package versions to retain in the backup directory.                                                                                                 |
| `debug_dir`         | String    | No       | -                                     | Absolute path to a directory where debug packages should be stored.                                                                                                   |
| `interactive`       | Boolean   | No       | `false`                               | If `true`, prompts for user confirmation before executing destructive actions like removing packages.                                                                 |
| `reference_repo`    | String    | No       | -                                     | Absolute path to a reference repository database. Used to compare and copy newer packages from another local repository.                                                |

---

## 2. PostgreSQL Integration

### Overview

When `postgresql_url` is configured, `repo-manage-util` connects to a PostgreSQL database to log every package addition, update, and removal. This creates a persistent, queryable data of your repositories.

### Schema

The database schema is automatically initialized and managed by the `pg_impl` crate. It includes tables for:

*   `repositories`: Stores information about each repository profile.
*   `packages`: Contains metadata for every package version ever added.
*   `repository_packages`: Links packages to repositories, tracking which version is currently in which repo.
*   And other related tables for dependencies, files, etc.

---

## 3. Deployment with Docker

The provided `docker-compose.yml` file makes it easy to deploy the PostgreSQL database and the API service.

### Prerequisites

*   [Docker](https://docs.docker.com/get-docker/)
*   [Docker Compose](https://docs.docker.com/compose/install/)

### Setup and Deployment Steps

1.  **Configure the API Service**:
    *   Navigate to the `api-service/` directory.
    *   Edit `config_vars-docker.yaml` and set the `db-connection` to match the credentials in the `docker-compose.yml` file. The default is: `postgresql://user:1234@repo-postgres:5432/pg_repomanage`.

2.  **Build and Start the Services**:
    *   From the root of the project, run the following command:
        ```bash
        docker-compose up -d --build
        ```

3.  **Verify Deployment**:
    *   Check the container logs: `docker-compose logs -f`
    *   The API service should be available at `http://localhost:5862`.

### API Service

The `api-service` provides RESTful endpoints to query the package data stored in the PostgreSQL database.

*   **Base URL**: `http://localhost:5862`
*   **API Documentation**: A Swagger/OpenAPI schema is available in `api-service/swagger.yaml`. This can be used with tools like Swagger UI for interactive documentation.
*   **Endpoints**:
    *   `/api/v1/packages-search`: Search for packages.
    *   `/api/v1/package/{repo}/{arch}/{pkgname}`: Get detailed information for a specific package.
