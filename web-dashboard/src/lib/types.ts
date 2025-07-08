/**
 * A brief representation of a package.
 */
export interface BriefPackage {
  /**
   * The name of the package.
   */
  pkg_name: string;
  /**
   * The name of the repository the package belongs to.
   */
  repo_name: string;
  /**
   * The architecture of the package.
   */
  pkg_arch: string;
  /**
   * The version of the package.
   */
  pkg_version: string;
  /**
   * A brief description of the package.
   */
  pkg_desc: string;
  /**
   * The timestamp (Unix epoch) when the package was last updated.
   */
  pkg_builddate: number;
}

/**
 * Detailed information for a specific package.
 */
export interface PackageDetails {
  repo_name: string;
  pkg_name: string;
  pkg_version: string;
  pkg_base: string | null;
  pkg_desc: string | null;
  pkg_groups: string[] | null;
  pkg_url: string | null;
  pkg_license: string[] | null;
  pkg_arch: string | null;
  pkg_builddate: number | null;
  pkg_packager: string | null;
  pkg_csize: number | null;
  pkg_isize: number | null;
  pkg_sha256sum: string | null;
  pkg_pgpsig: string | null;
  pkg_replaces: string[] | null;
  pkg_depends: string[] | null;
  pkg_optdepends: string[] | null;
  pkg_makedepends: string[] | null;
  pkg_checkdepends: string[] | null;
  pkg_conflicts: string[] | null;
  pkg_provides: string[] | null;
  pkg_files: string[] | null;
  updated: number;
}

/**
 * The response schema for a package search request.
 */
export interface PackageSearchResponse {
  /**
   * The total number of packages matching the search criteria.
   */
  total_packages: number;
  /**
   * The total number of pages available.
   */
  total_pages: number;
  /**
   * An array of packages matching the search criteria for the current page.
   */
  packages: BriefPackage[];
}

/**
 * Query parameters for the package search endpoint.
 */
export interface PackagesSearchQueryParams {
  /**
   * The search term to find packages by name or description.
   */
  search?: string;
  /**
   * A comma-separated list of repository names to filter by.
   * @example "my-repo-1,my-repo-2"
   */
  repo?: string;
  /**
   * A comma-separated list of architectures to filter by.
   * @example "x86_64,aarch64"
   */
  arch?: string;
  /**
   * The page number to retrieve.
   * @default 1
   */
  current_page?: number;
  /**
   * The number of packages to return per page.
   * @default 100
   */
  page_size?: number;
}

/**
 * Path parameters for getting package details.
 */
export interface PackageDetailsPathParams {
  /**
   * The name of the repository.
   * @example "my-stable-repo"
   */
  repo: string;
  /**
   * The architecture of the package.
   * @example "x86_64"
   */
  arch: string;
  /**
   * The name of the package.
   * @example "openssl"
   */
  pkgname: string;
}

/**
 * The response body for a successful package details request.
 */
export interface PackageDetailsResponse {
  package: PackageDetails;
}
