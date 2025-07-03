# API Documentation for repo-manage-util API Service

This document provides details on the deployment, configuration, and API endpoints for the `repo-manage-util` API service.

## Deployment

The API service is designed to be run as a Docker container. The project includes a `Dockerfile` for building the service and a `docker-compose.yml` file for orchestrating the service along with its PostgreSQL database dependency.

### Building and Running with Docker Compose (Recommended)

1.  **Prerequisites:** Ensure you have Docker and Docker Compose installed on your system.
2.  **Navigate to the root of the repository.**
3.  **Build and start the services:**
    ```bash
    docker-compose up --build -d
    ```
    This command will:
    *   Build the API service image using `api-service/Dockerfile`.
    *   Start the API service container.
    *   Start a PostgreSQL container (`repo-postgres`) if it's not already running.
    *   The API service will be available on port `5862` by default on the host machine.

### Building with Dockerfile manually

1.  **Navigate to the `api-service` directory:**
    ```bash
    cd api-service
    ```
2.  **Build the Docker image:**
    ```bash
    docker build -t repo-manage-api .
    ```
3.  **Run the Docker container:**
    You will need to ensure a PostgreSQL instance is running and accessible to the container. You'll also need to provide the necessary configuration, typically via a config file or environment variables.
    ```bash
    docker run -p 5862:5862 \
           -e DB_CONNECTION_STRING="postgresql://user:password@your_postgres_host:5432/pg_repomanage" \
           repo-manage-api
    ```
    *(Note: Adjust the port mapping and environment variables as needed.)*

The service listens on port `5862` by default, as defined in the `api-service/Dockerfile` (EXPOSE 5862) and `docker-compose.yml`.
Refer to `api-service/Dockerfile` for build specifics and `docker-compose.yml` for service orchestration details.

## Configuration

The API service is configured primarily through YAML files.

*   **`api-service/config_vars.yaml`**: This is the main configuration file when running the service outside of Docker or for local development.
    *   `logger-level`: Sets the logging verbosity (e.g., `info`, `debug`, `trace`).
    *   `server-port`: The port on which the HTTP server will listen (default: `5862`).
    *   `testsuite-enabled`: A boolean flag to enable or disable testsuite-specific handlers and behavior.
    *   `enabled-without-testsuite`: Likely related to enabling certain features when the testsuite is not active.
    *   `db-connection`: The PostgreSQL connection string. Format: `postgresql://user:password@host:port/dbname`.
    *   `config-fallback`: Path to a fallback configuration file (`dynamic_config_fallback.json`).

*   **`api-service/config_vars-docker.yaml`**: This file provides overrides for when the service is run inside a Docker container, particularly when managed by `docker-compose.yml`. The `CMD` in the `api-service/Dockerfile` specifies this config:
    `CMD [ "/app/api-postgresql-service", "--config", "postgres_service.yaml", "--config_vars", "config_vars-docker.yaml"]`
    It's important to check this file for any differences from `config_vars.yaml` in a Docker deployment. For example, the `db-connection` string will likely point to the Docker service name for PostgreSQL (e.g., `repo-postgres` as defined in `docker-compose.yml`).

*   **`api-service/postgres_service.yaml`**: This is the static configuration for the userver framework, defining components, their settings, and HTTP handlers. The actual values for variables like database connection strings are typically sourced from `config_vars.yaml` or `config_vars-docker.yaml`.

*   **`api-service/dynamic_config_fallback.json`**: This file contains fallback values for dynamic configuration options that the service might try to fetch from a configuration server. If the server is unavailable, these values are used.

Key configurations to be aware of:
*   **Database Connection:** Ensure the `db-connection` string is correctly set to point to your PostgreSQL instance. In the `docker-compose` setup, this is handled by using the service name `repo-postgres`.
*   **Server Port:** The port the service listens on, default `5862`.

## API Endpoints

The API is specified using OpenAPI 3.0.0.
The service handlers in `main.cpp` (`PackageHandler`, `PackagesSearchHandler`) are registered without an explicit versioned base path like `/v1`. Assuming they are served from the root.

```yaml
openapi: 3.0.0
info:
  title: Repo Manage Util API
  version: "1.0.0"
  description: API for managing and searching packages in Arch Linux repositories.
servers:
  - url: http://localhost:5862
    description: Local development server

paths:
  /package/{repo}/{arch}/{pkgname}:
    get:
      summary: Get specific package details
      description: Retrieves detailed information about a specific package identified by repository, architecture, and package name.
      operationId: getPackageDetails
      parameters:
        - name: repo
          in: path
          required: true
          description: Name of the repository (e.g., 'core', 'extra').
          schema:
            type: string
        - name: arch
          in: path
          required: true
          description: Architecture of the package (e.g., 'x86_64', 'any').
          schema:
            type: string
        - name: pkgname
          in: path
          required: true
          description: Name of the package.
          schema:
            type: string
      responses:
        '200':
          description: Successfully retrieved package details.
          content:
            application/json:
              schema:
                type: object
                properties:
                  package: # Matches the structure returned by PackageHandler
                    $ref: '#/components/schemas/PublicPackage'
        '404':
          description: Package not found.
          content:
            application/json:
              schema:
                type: object
                properties:
                  code:
                    type: string
                    example: RESOURCE_NOT_FOUND
                  message:
                    type: string
                    example: Package not found!

  /packages/search:
    get:
      summary: Search for packages
      description: Searches for packages based on query parameters, with support for filtering and pagination.
      operationId: searchPackages
      parameters:
        - name: current_page
          in: query
          description: The page number to retrieve.
          required: false
          schema:
            type: integer
            default: 1
            minimum: 1
        - name: page_size
          in: query
          description: The number of packages to return per page.
          required: false
          schema:
            type: integer
            default: 100
            minimum: 1
        - name: search
          in: query
          description: Search query string to match against package names and descriptions.
          required: false
          schema:
            type: string
        - name: repo
          in: query
          description: Comma-separated list of repository names to filter by (e.g., 'core,extra').
          required: false
          schema:
            type: string
        - name: arch
          in: query
          description: Comma-separated list of architectures to filter by (e.g., 'x86_64,any').
          required: false
          schema:
            type: string
      responses:
        '200':
          description: Successfully retrieved search results.
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/PackagesSearchResult'
        '400':
          description: Invalid query parameters (e.g., current_page or page_size <= 0).
          content:
            application/json:
              schema:
                type: object
                properties:
                  code:
                    type: string
                    example: CLIENT_ERROR
                  message:
                    type: string
                    example: Invalid page!

components:
  schemas:
    PublicPackage:
      type: object
      description: Detailed information about a software package.
      properties:
        repo_name:
          type: string
          description: Name of the repository the package belongs to.
        pkg_name:
          type: string
          description: Name of the package.
        pkg_version:
          type: string
          description: Version of the package.
        pkg_base:
          type: string
          nullable: true
          description: The base package name, if different from pkg_name (e.g., for split packages).
        pkg_desc:
          type: string
          nullable: true
          description: Description of the package.
        pkg_groups:
          type: array
          items:
            type: string
          nullable: true
          description: Groups the package belongs to.
        pkg_url:
          type: string
          nullable: true
          description: Upstream URL of the package.
        pkg_license:
          type: array
          items:
            type: string
          nullable: true
          description: Licenses of the package.
        pkg_arch:
          type: string
          nullable: true
          description: Architecture of the package.
        pkg_builddate:
          type: integer
          format: int64 # Unix timestamp (seconds since epoch)
          nullable: true
          description: Build date of the package as a Unix timestamp.
        pkg_packager:
          type: string
          nullable: true
          description: Name and email of the package packager.
        pkg_csize:
          type: integer
          format: int64
          nullable: true
          description: Compressed size of the package in bytes.
        pkg_isize:
          type: integer
          format: int64
          nullable: true
          description: Installed size of the package in bytes.
        pkg_sha256sum:
          type: string
          nullable: true
          description: SHA256 checksum of the package file.
        pkg_pgpsig:
          type: string
          nullable: true
          description: Base64 encoded PGP signature of the package file.
        pkg_replaces:
          type: array
          items:
            type: string
          nullable: true
          description: List of packages this package replaces.
        pkg_depends:
          type: array
          items:
            type: string
          nullable: true
          description: List of packages this package depends on.
        pkg_optdepends:
          type: array
          items:
            type: string
          nullable: true
          description: List of optional dependencies for this package.
        pkg_makedepends:
          type: array
          items:
            type: string
          nullable: true
          description: List of packages required to build this package.
        pkg_checkdepends:
          type: array
          items:
            type: string
          nullable: true
          description: List of packages required to run the test suite for this package.
        pkg_conflicts:
          type: array
          items:
            type: string
          nullable: true
          description: List of packages that conflict with this package.
        pkg_provides:
          type: array
          items:
            type: string
          nullable: true
          description: List of virtual packages or capabilities provided by this package.
        pkg_files:
          type: array
          items:
            type: string
          nullable: true
          description: List of files included in this package.
        updated:
          type: integer
          format: int64 # Unix timestamp (seconds since epoch)
          description: Last update timestamp for this package entry as a Unix timestamp.
      required:
        - repo_name
        - pkg_name
        - pkg_version
        - updated

    BriefPackage:
      type: object
      description: Brief information about a software package, typically used in search results.
      properties:
        pkg_name:
          type: string
          description: Name of the package.
        repo_name:
          type: string
          description: Name of the repository the package belongs to.
        pkg_arch:
          type: string
          description: Architecture of the package.
        pkg_version:
          type: string
          description: Version of the package.
        pkg_desc:
          type: string
          description: Description of the package.
        updated:
          type: integer
          format: int64 # Unix timestamp (seconds since epoch)
          description: Last update timestamp for this package entry as a Unix timestamp.
      required:
        - pkg_name
        - repo_name
        - pkg_arch
        - pkg_version
        - pkg_desc
        - updated

    PackagesSearchResult:
      type: object
      description: Result of a package search operation.
      properties:
        total_packages:
          type: integer
          format: int64
          description: Total number of packages matching the search criteria.
        packages:
          type: array
          items:
            $ref: '#/components/schemas/BriefPackage'
          description: List of packages found in the current page.
        total_pages: # Added based on handler logic
          type: integer
          format: int64
          description: Total number of pages available for the search criteria.
      required:
        - total_packages
        - packages
        - total_pages
```
