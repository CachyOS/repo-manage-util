import json
import os
import pathlib
import sys

import pytest

from testsuite.databases.pgsql import discover

import __main__

sys.path.append(os.path.join(os.path.dirname(__file__), 'helpers'))

pytest_plugins = ['pytest_userver.plugins.core', 'pytest_userver.plugins.postgresql']

@pytest.fixture(scope='session')
def service_source_dir():
    """Path to root directory service."""
    return pathlib.Path(__file__).parent.parent


@pytest.fixture(scope='session')
def initial_data_path(service_source_dir):
    """Path for find files with data"""
    return [
        service_source_dir / 'postgresql/data',
    ]

@pytest.fixture(scope='session')
def pgsql_local(service_source_dir, pgsql_local_create):
    """Create schemas databases for tests"""
    databases = discover.find_schemas(
        'api-postgresql-service',  # service name that goes to the DB connection
        [service_source_dir.joinpath('postgresql/schemas')],
    )
    return pgsql_local_create(list(databases.values()))

@pytest.fixture(
    autouse=True,
    params=[0, 1],
    ids=['pipeline_disabled', 'pipeline_enabled'],
)
async def pipeline_mode(request, service_client, dynamic_config):
    dynamic_config.set_values({
        'POSTGRES_CONNECTION_PIPELINE_EXPERIMENT': request.param,
    })
    await service_client.update_server_state()
