import pytest

from testsuite.databases import pgsql
from endpoints import *

dolt_package = {'repo_name': 'repo1', 'pkg_name': 'dolt', 'pkg_version': '1.30.5-1.1', 'pkg_base': 'dolt', 'pkg_desc': 'Git for data! A version controlled relational database', 'pkg_groups': [], 'pkg_url': 'https://www.dolthub.com', 'pkg_license': ['Apache'], 'pkg_arch': 'x86_64', 'pkg_builddate': 1704385114, 'pkg_packager': 'CachyOS <admin@cachyos.org>', 'pkg_csize': 18527187, 'pkg_isize': 93412792, 'pkg_sha256sum': '36f26a9a520800f3a2c577f07334ca1c209bcd3ac6c457108abd417ec242d299', 'pkg_pgpsig': None, 'pkg_replaces': [], 'pkg_depends': ['glibc'], 'pkg_optdepends': [], 'pkg_makedepends': ['go'], 'pkg_checkdepends': [], 'pkg_conflicts': [], 'pkg_provides': [], 'pkg_files': [], 'updated': 1751364311}
dolt_pkgfiles = 'usr/,usr/bin/,usr/bin/dolt'.split(',')

@pytest.mark.pgsql('init_db', files=['initial_data.sql'])
async def test_package(service_client):
    response = await get_package(service_client)
    assert response.status == 404

    response = await get_package(service_client, 'day')
    assert response.status == 404

    response = await get_package(service_client, 'repo2', 'x86_64', 'dolt')
    assert response.status == 404

    response = await get_package(service_client, 'repo1', 'x86_64_v3', 'dolt')
    assert response.status == 404

    response = await get_package(service_client, 'repo1', 'x86_64', 'dolt1')
    assert response.status == 404

    response = await get_package_files(service_client, 'repo1', 'x86_64', 'dolt1')
    assert response.status == 404

    response = await get_package(service_client, 'repo1', 'x86_64', 'dolt')
    assert response.status == 200
    assert response.json()['package'] == dolt_package

    response = await get_package_files(service_client, 'repo1', 'x86_64', 'dolt')
    assert response.status == 200
    assert response.json() == dolt_pkgfiles

@pytest.mark.pgsql('init_db', files=['initial_data.sql'])
async def test_search_packages(service_client):
    # invalid usage
    response = await search_packages(service_client, 0, 0)
    assert response.status == 400

    response = await search_packages(service_client, 0, 20)
    assert response.status == 400

    response = await search_packages(service_client, 0, -2)
    assert response.status == 400

    response = await search_packages(service_client, -2, 20)
    assert response.status == 400

    # valid usage
    response = await search_packages(service_client, 1, 100, 'dolt')
    assert response.status == 200
    assert len(response.json()['packages']) == 1
    assert response.json()['total_pages'] == 1
    assert [pkg['pkg_name']+'+'+pkg['pkg_arch'] for pkg in response.json()['packages']] == ['dolt+x86_64']

    response = await search_packages(service_client, 1, 100, 'dolt', 'repo1', 'x86_64')
    assert response.status == 200
    assert len(response.json()['packages']) == 1
    assert response.json()['total_pages'] == 1
    assert [pkg['pkg_name']+'+'+pkg['pkg_arch'] for pkg in response.json()['packages']] == ['dolt+x86_64']

    response = await search_packages(service_client, 1, 100, 'dolt', 'repo1', 'x86_64_v3')
    assert response.status == 200
    assert len(response.json()['packages']) == 0
    assert response.json()['total_pages'] == 0

    response = await search_packages(service_client, query='a')
    assert response.status == 200
    assert len(response.json()['packages']) == 13

@pytest.mark.pgsql('init_db', files=['initial_data.sql', 'split_data.sql'])
async def test_split_packages(service_client):
    response = await get_split_package(service_client, 'test1', 'opencv')
    assert response.status == 404

    response = await get_split_package(service_client, 'repo1', 'uv')
    assert response.status == 404

    response = await get_split_package(service_client, 'test1', 'godot')
    assert response.status == 200
    assert len(response.json()) == 2
    assert [pkg['pkg_name'] for pkg in response.json()] == ['godot-mono', 'godot']

    response = await get_split_package(service_client, 'test1', 'uv')
    assert response.status == 200
    assert len(response.json()) == 3
    assert [pkg['pkg_name'] for pkg in response.json()] == ['python-uv-build', 'uv', 'python-uv']

@pytest.mark.pgsql('init_db', files=['initial_data.sql'])
async def test_suggest_packages(service_client):
    response = await get_packages_suggest(service_client, query='d')
    assert response.headers['Content-Type'] == 'application/x-suggestions+json'
    assert response.status == 200
    suggestions = response.json()
    assert len(suggestions) == 2
    query = suggestions[0]
    names = suggestions[1]
    assert query == 'd'
    assert names == ['docker', 'dolt', 'dwl-git', 'dwm']

    response = await get_packages_suggest(service_client, limit=2, query='d')
    suggestions = response.json()
    assert len(suggestions) == 2
    query = suggestions[0]
    names = suggestions[1]
    assert query == 'd'
    assert names == ['docker', 'dolt']

    response = await get_packages_suggest(service_client, query='nginx')
    suggestions = response.json()
    assert len(suggestions) == 2
    query = suggestions[0]
    names = suggestions[1]
    assert query == 'nginx'
    assert names == ['nginx']

    response = await get_packages_suggest(service_client, query='unknown')
    assert response.status == 200
    suggestions = response.json()
    assert len(suggestions) == 2
    query = suggestions[0]
    names = suggestions[1]
    assert query == 'unknown'
    assert names == []
