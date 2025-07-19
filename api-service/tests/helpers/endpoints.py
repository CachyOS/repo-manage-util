async def get_package(service_client, repo: str = '', arch: str = '', pkgname: str = ''):
  return await service_client.get(
    f'/api/v1/package/{repo}/{arch}/{pkgname}',
  )

async def search_packages(service_client, current_page: int = 1, page_size: int = 100, query: str = '', repo_filter: str = '', arch_filter: str = ''):
  return await service_client.get(
    f'/api/v1/packages-search?current_page={current_page}&page_size={page_size}&search={query}&repo={repo_filter}&arch={arch_filter}',
  )

async def get_split_package(service_client, repo: str, pkgbase: str):
  return await service_client.get(
    f'/api/v1/split/{repo}/{pkgbase}',
  )

