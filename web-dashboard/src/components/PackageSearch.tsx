'use client';

import {
  PackageSearchResponse,
  PackagesSearchQueryParams,
} from '@/lib/types';
import { searchPackages } from '@/lib/actions';
import { useState, useEffect } from 'react';
import Link from 'next/link';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table';
import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert';
import { Loader2, AlertCircle } from 'lucide-react';

export default function PackageSearch() {
  const [params, setParams] = useState<PackagesSearchQueryParams>({
    search: '',
    repo: '',
    arch: '',
    current_page: 1,
    page_size: 15,
  });

  const [results, setResults] = useState<PackageSearchResponse | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleSearch = async (page = 1) => {
    setIsLoading(true);
    setError(null);

    const searchParams = { ...params, current_page: page };

    try {
      const response = await searchPackages(searchParams);
      setResults(response);
      // Update current_page in state after a successful search
      setParams(searchParams);
    } catch (err) {
      console.error(err);
      setError('Failed to fetch packages. Please try again later.');
      setResults(null);
    } finally {
      setIsLoading(false);
    }
  };

  // load search packages without params on page load
  useEffect(() => {
    setIsLoading(true);
    handleSearch();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const onFormSubmit = (e: React.FormEvent<HTMLFormElement>) => {
    e.preventDefault();

    // reset page
    handleSearch();
  };

  const onInputChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const { name, value } = e.target;
    setParams(prev => ({ ...prev, [name]: value }));
  };

  return (
    <div className="space-y-8">
      {/* Search Form */}
      <form onSubmit={onFormSubmit} className="space-y-4">
        <div className="grid grid-cols-1 gap-4 md:grid-cols-3">
          <div className="space-y-2">
            <Label htmlFor="search">Package Name/Description</Label>
            <Input
              id="search"
              name="search"
              placeholder="e.g., openssl"
              value={params.search}
              onChange={onInputChange}
            />
          </div>
          <div className="space-y-2">
            <Label htmlFor="repo">Repositories (comma-separated)</Label>
            <Input
              id="repo"
              name="repo"
              placeholder="e.g., my-stable-repo"
              value={params.repo}
              onChange={onInputChange}
            />
          </div>
          <div className="space-y-2">
            <Label htmlFor="arch">Architectures (comma-separated)</Label>
            <Input
              id="arch"
              name="arch"
              placeholder="e.g., x86_64,aarch64"
              value={params.arch}
              onChange={onInputChange}
            />
          </div>
        </div>
        <Button type="submit" disabled={isLoading}>
          {isLoading ? (
            <>
              <Loader2 className="mr-2 h-4 w-4 animate-spin" />
              Searching...
            </>
          ) : (
            'Search'
          )}
        </Button>
      </form>

      {/* Results Section */}
      <div className="results-area">
        {error && (
          <Alert variant="destructive">
            <AlertCircle className="h-4 w-4" />
            <AlertTitle>Error</AlertTitle>
            <AlertDescription>{error}</AlertDescription>
          </Alert>
        )}

        {results && (
          <div className="space-y-4">
            <p className="text-sm text-muted-foreground">
              Found {results.total_packages.toLocaleString()} packages. Page{' '}
              {params.current_page} of {results.total_pages.toLocaleString()}.
            </p>

            {results.packages.length > 0 ? (
              <>
                <div className="rounded-md border">
                  <Table>
                    <TableHeader>
                      <TableRow>
                        <TableHead>Package Name</TableHead>
                        <TableHead>Version</TableHead>
                        <TableHead>Repository</TableHead>
                        <TableHead>Architecture</TableHead>
                        <TableHead>Last Updated</TableHead>
                      </TableRow>
                    </TableHeader>
                    <TableBody>
                      {results.packages.map(pkg => (
                        <TableRow key={`${pkg.repo_name}-${pkg.pkg_name}-${pkg.pkg_arch}`}>
                          <TableCell className="font-medium">
                            <Link
                              href={`/package/${encodeURIComponent(pkg.repo_name)}/${encodeURIComponent(pkg.pkg_arch)}/${encodeURIComponent(pkg.pkg_name)}`}
                              className="text-primary hover:underline"
                            >
                              {pkg.pkg_name}
                            </Link>
                          </TableCell>
                          <TableCell>{pkg.pkg_version}</TableCell>
                          <TableCell>{pkg.repo_name}</TableCell>
                          <TableCell>{pkg.pkg_arch}</TableCell>
                          <TableCell>
                            {new Date(pkg.updated * 1000).toLocaleDateString()}
                          </TableCell>
                        </TableRow>
                      ))}
                    </TableBody>
                  </Table>
                </div>
                {/* Pagination Controls */}
                <div className="flex items-center justify-end space-x-2">
                  <Button
                    variant="outline"
                    size="sm"
                    onClick={() => handleSearch(params.current_page! - 1)}
                    disabled={isLoading || params.current_page! <= 1}
                  >
                    Previous
                  </Button>
                  <Button
                    variant="outline"
                    size="sm"
                    onClick={() => handleSearch(params.current_page! + 1)}
                    disabled={isLoading || params.current_page! >= results.total_pages}
                  >
                    Next
                  </Button>
                </div>
              </>
            ) : (
              <p className="text-center text-muted-foreground">
                No packages found matching your criteria.
              </p>
            )}
          </div>
        )}
      </div>
    </div>
  );

  /*
  useEffect(() => {
    const fetchPackages = async () => {
      try {
        const response = await fetch(`localhost:5862/api/v1/packages-search?search=${search}`);
        if (!response.ok) {
          throw new Error("Failed to fetch packages");
        }
        const data = await response.json();
        setPackages(data.packages);
      } catch (err) {
        setError(err.message);
      }
    };

    const debounce = setTimeout(() => {
      if (search) {
        fetchPackages();
      } else {
        setPackages([]);
      }
    }, 500);

    return () => clearTimeout(debounce);
  }, [search]);
  return (
    <div className="container mx-auto p-4">
      <input
        type="text"
        value={search}
        onChange={(e) => setSearch(e.target.value)}
        placeholder="Search for packages..."
        className="w-full p-2 border rounded"
      />
      {error && <p className="text-red-500">{error}</p>}
      <table className="w-full mt-4 text-left table-auto">
        <thead>
          <tr>
            <th className="px-4 py-2">Name</th>
            <th className="px-4 py-2">Version</th>
            <th className="px-4 py-2">Repository</th>
            <th className="px-4 py-2">Architecture</th>
            <th className="px-4 py-2">Description</th>
          </tr>
        </thead>
        <tbody>
          {packages.map((pkg) => (
            <tr key={`${pkg.repo_name}/${pkg.pkg_name}`}>
              <td className="border px-4 py-2">{pkg.pkg_name}</td>
              <td className="border px-4 py-2">{pkg.pkg_version}</td>
              <td className="border px-4 py-2">{pkg.repo_name}</td>
              <td className="border px-4 py-2">{pkg.pkg_arch}</td>
              <td className="border px-4 py-2">{pkg.pkg_desc}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );*/
}
