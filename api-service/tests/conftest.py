import json
import os
import pathlib
import sys

import pytest
from pytest_userver import chaos

from testsuite.databases.pgsql import discover

import __main__

sys.path.append(os.path.join(os.path.dirname(__file__), 'helpers'))

pytest_plugins = ['pytest_userver.plugins.core', 'pytest_userver.plugins.postgresql', 'pytest_userver.plugins.redis']

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

@pytest.fixture(scope='session')
def userver_testsuite_middleware_enabled():
    return False

@pytest.fixture(scope='session')
async def _gate_started(pgsql_local):
    gate_config = chaos.GateRoute(
        name='postgres proxy',
        host_to_server=pgsql_local['init_db'].host,
        port_to_server=pgsql_local['init_db'].port,
    )
    async with chaos.TcpGate(gate_config) as proxy:
        yield proxy

@pytest.fixture
def extra_client_deps(_gate_started):
    pass

# /// [gate fixture]
@pytest.fixture(name='gate')
async def _gate_ready(service_client, _gate_started):
    await _gate_started.to_server_pass()
    await _gate_started.to_client_pass()
    _gate_started.start_accepting()

    await _gate_started.wait_for_connections()
    yield _gate_started
    # /// [gate fixture]

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

@pytest.fixture(scope='session')
def service_secdist_path(service_source_dir):
    return service_source_dir / 'secure_data.testing.json'

# /// [service_env]
@pytest.fixture(scope='session')
def service_env(redis_sentinels):
    secdist_config = {
        'redis_settings': {
            'redis-sentinel': {
                'password': '',
                'database_index': 0,
                'sentinels': redis_sentinels,
                'shards': [{'name': 'test_master0'}],
            },
        },
    }

    return {'SECDIST_CONFIG': json.dumps(secdist_config)}
    # /// [service_env]
