async def test_ping(service_client):
    response = await service_client.get('/ping')
    assert response.status == 200

    # Tracing headers should be present
    assert 'X-YaRequestId' in response.headers.keys()
    assert 'X-YaTraceId' in response.headers.keys()
    assert 'X-YaSpanId' in response.headers.keys()

async def test_monitor(monitor_client):
    response = await monitor_client.get('/service/log-level/')
    # shouldn't be enabled with current setup
    assert response.status == 404

async def test_metrics_smoke(monitor_client):
    metrics = await monitor_client.metrics()
    assert len(metrics) > 1

async def test_partial_metrics_portability(service_client):
    warnings = await service_client.metrics_portability()
    warnings.pop('label_name_mismatch', None)
    assert not warnings, warnings
