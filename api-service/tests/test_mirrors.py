from endpoints import get_mirrors

def _register_lastupdate_handler(mockserver, prefix, values):
    @mockserver.handler(prefix, prefix=True)
    def _handler(request):
        repo_path = request.path.removeprefix(f'{prefix}/').removesuffix('/lastupdate')
        value = values.get(repo_path)
        if value is None:
            return mockserver.make_response('', 404)
        return mockserver.make_response(str(value), 200)


async def test_get_mirrors(service_client, mockserver):
    primary_values = {
        'x86_64/cachyos': 172_800_000,
        'x86_64_v3/cachyos-v3': 259_200_000,
        'x86_64_v4/cachyos-v4': 345_600_000,
    }
    mirror_a_values = {
        'x86_64/cachyos': 172_800_000,
        'x86_64_v3/cachyos-v3': 259_200_000,
        'x86_64_v4/cachyos-v4': 345_600_000,
    }
    mirror_b_values = {
        'x86_64/cachyos': 86_400_000,
        'x86_64_v3/cachyos-v3': 259_200_000,
        'x86_64_v4/cachyos-v4': 345_600_000,
    }
    mirror_c_values = {
        'x86_64_v3/cachyos-v3': 259_200_000,
        'x86_64_v4/cachyos-v4': 345_600_000,
    }
    mirror_e_values = {
        'x86_64/cachyos': 172_800_000,
        'x86_64_v3/cachyos-v3': 259_200_000,
        'x86_64_v4/cachyos-v4': 345_600_000,
    }

    @mockserver.handler('/mirrors/list')
    def _mirrorlist(_request):
        return mockserver.make_response(
            '\n'.join([
                '## France Mirror',
                '## tier=1 code=FR',
                f'Server = {mockserver.url("/mirror-a/repo/$arch/$repo")}',
                '## USA Mirror',
                '## tier=2 code=US',
                'Server =',
                mockserver.url('/mirror-b/repo/$arch/$repo'),
                '## Norway Mirror',
                '## tier=1 code=NO',
                'Server =',
                mockserver.url('/mirror-c/repo/$arch/$repo'),
                '## Germany Mirror',
                '## tier=1 code=DE',
                '# Server =',
                mockserver.url('/mirror-d/repo/$arch/$repo'),
                '## Finland Mirror',
                '## tier=2 code=FI',
                mockserver.url('/mirror-e/repo/$arch/$repo'),
            ]),
            200,
        )

    _register_lastupdate_handler(mockserver, '/primary', primary_values)
    _register_lastupdate_handler(mockserver, '/mirror-a', mirror_a_values)
    _register_lastupdate_handler(mockserver, '/mirror-b', mirror_b_values)
    _register_lastupdate_handler(mockserver, '/mirror-c', mirror_c_values)
    _register_lastupdate_handler(mockserver, '/mirror-e', mirror_e_values)

    await service_client.invalidate_caches(cache_names=['mirrors-data-cache'])

    response = await get_mirrors(service_client)
    assert response.status == 200

    payload = response.json()
    assert payload.keys() == {'mirrors'}

    mirrors_by_url = {mirror['url']: mirror for mirror in payload['mirrors']}
    assert set(mirrors_by_url) == {
        mockserver.url('/mirror-a/'),
        mockserver.url('/mirror-b/'),
        mockserver.url('/mirror-c/'),
        mockserver.url('/mirror-e/'),
    }

    assert mirrors_by_url[mockserver.url('/mirror-a/')] == {
        'country_code': 'FR',
        'url': mockserver.url('/mirror-a/'),
        'out_of_date': False,
        'last_sync': '1970-01-03T00:00:00Z',
        'tier': 1,
    }
    assert mirrors_by_url[mockserver.url('/mirror-b/')] == {
        'country_code': 'US',
        'url': mockserver.url('/mirror-b/'),
        'out_of_date': True,
        'last_sync': '1970-01-02T00:00:00Z',
        'tier': 2,
    }
    assert mirrors_by_url[mockserver.url('/mirror-c/')] == {
        'country_code': 'NO',
        'url': mockserver.url('/mirror-c/'),
        'out_of_date': True,
        'last_sync': '1970-01-04T00:00:00Z',
        'tier': 1,
    }
    assert mirrors_by_url[mockserver.url('/mirror-e/')] == {
        'country_code': 'FI',
        'url': mockserver.url('/mirror-e/'),
        'out_of_date': False,
        'last_sync': '1970-01-03T00:00:00Z',
        'tier': 2,
    }
