from ipaddress import ip_address

from bowbend import Builder, start_scan, Target
import pytest


@pytest.mark.timeout(10)
@pytest.mark.asyncio
async def test_start_scan():
    ipv4_target = Target(ip_address("127.0.0.1"))
    builder = Builder()
    builder.add_target(ipv4_target)
    builder.set_tracing(True)
    await start_scan(builder)
