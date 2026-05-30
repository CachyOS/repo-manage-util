from endpoints import get_mirrors_data


REPO_PATHS = [
    'x86_64/cachyos',
    'x86_64_v3/cachyos-v3',
    'x86_64_v4/cachyos-v4',
]


def _register_lastupdate_handler(mockserver, prefix, values):
    @mockserver.handler(prefix, prefix=True)
    def _handler(request):
        repo_path = request.path.removeprefix(f'{prefix}/').removesuffix('/lastupdate')
        value = values.get(repo_path)
        if value is None:
            return mockserver.make_response('', 404)
        return mockserver.make_response(str(value), 200)


async def test_get_mirrors_data(service_client, mockserver):
    primary_values = {
        'x86_64/cachyos': 5_000_000,
        'x86_64_v3/cachyos-v3': 8_000_000,
    }
    mirror_a_values = {
        'x86_64/cachyos': 4_700_000,
        'x86_64_v3/cachyos-v3': 7_900_000,
        'x86_64_v4/cachyos-v4': 9_500_000,
    }
    mirror_b_values = {
        'x86_64/cachyos': 100_000,
        'x86_64_v4/cachyos-v4': 9_100_000,
    }
    mirror_c_values = {
        'x86_64_v3/cachyos-v3': 1_000_000,
    }
    mirror_d_values = {}

    @mockserver.handler('/mirrors/list')
    def _mirrorlist(_request):
        return mockserver.make_response(
            '\n'.join([
                '# comment',
                f'Server = {mockserver.base_url}/mirror-b/$repo/$arch',
                f'{mockserver.base_url}/mirror-a',
                f'Server = {mockserver.base_url}/mirror-c/',
                f'Server = {mockserver.base_url}/mirror-d/$repo',
            ]),
            200,
        )

    _register_lastupdate_handler(mockserver, '/primary', primary_values)
    _register_lastupdate_handler(mockserver, '/mirror-a', mirror_a_values)
    _register_lastupdate_handler(mockserver, '/mirror-b', mirror_b_values)
    _register_lastupdate_handler(mockserver, '/mirror-c', mirror_c_values)
    _register_lastupdate_handler(mockserver, '/mirror-d', mirror_d_values)

    await service_client.invalidate_caches(cache_names=['mirrors-data-cache'])

    response = await get_mirrors_data(service_client)
    assert response.status == 200

    payload = response.json()
    assert payload['baselines'] == [
        {'path': REPO_PATHS[0], 'timestamp': 5000},
        {'path': REPO_PATHS[1], 'timestamp': 8000},
        {'path': REPO_PATHS[2], 'timestamp': None},
    ]

    assert [mirror['overallStatus'] for mirror in payload['mirrors']] == [
        'healthy',
        'partial',
        'out-of-sync',
        'error',
    ]

    healthy = payload['mirrors'][0]
    assert healthy['url'] == f'{mockserver.base_url}/mirror-a'
    assert healthy['averageLagSeconds'] == 200.0
    assert [check['status'] for check in healthy['checks']] == ['synced', 'synced', 'synced']
    assert healthy['checks'][2]['syncLagSeconds'] is None

    partial = payload['mirrors'][1]
    assert partial['url'] == f'{mockserver.base_url}/mirror-b'
    assert partial['averageLagSeconds'] == 4900.0
    assert [check['status'] for check in partial['checks']] == ['out-of-sync', 'error', 'synced']

    out_of_sync = payload['mirrors'][2]
    assert out_of_sync['url'] == f'{mockserver.base_url}/mirror-c'
    assert out_of_sync['averageLagSeconds'] == 7000.0
    assert [check['status'] for check in out_of_sync['checks']] == ['error', 'out-of-sync', 'error']

    errored = payload['mirrors'][3]
    assert errored['url'] == f'{mockserver.base_url}/mirror-d'
    assert errored['averageLagSeconds'] is None
    assert [check['status'] for check in errored['checks']] == ['error', 'error', 'error']