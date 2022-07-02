from ipaddress import ip_address

from bowbend import Builder, Scan, Target
import pytest


@pytest.mark.timeout(10)
@pytest.mark.asyncio
async def test_start_scan():

    ipv4_target = Target(ip_address("127.0.0.1"))
    builder = Builder()
    builder.set_ping(False)
    builder.add_target(ipv4_target)
    builder.set_tracing(True)
    scan = Scan(builder)
    x = await scan.next()
    print("PYTHON got result: " + str(x))
    x = await scan.next()
    print("PYTHON got result: " + str(x))

    raise 1

