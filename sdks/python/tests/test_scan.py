from ipaddress import ip_address

from bowbend import Builder, Scan, Target
import pytest

from bowbend.report import PortStatus
from bowbend.scan import ScanFinished


@pytest.mark.timeout(10)
@pytest.mark.asyncio
async def test_start_scan():

    ipv4_target = Target(ip_address("127.0.0.1"))
    builder = Builder()
    builder.set_ping(False)
    builder.set_port_list([80])
    builder.add_target(ipv4_target)
    builder.set_tracing(True)
    scan = Scan(builder)
    result = await scan.next()
    assert type(await scan.next()) is ScanFinished
    assert result.contents.ports.get(80).status == PortStatus.Closed
