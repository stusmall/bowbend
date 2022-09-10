from ipaddress import ip_address
from os import environ

from bowbend import Builder, Scan, Target
import pytest

from bowbend.report import PortStatus
from bowbend.scan import ScanFinished


@pytest.mark.timeout(10)
@pytest.mark.skipif(environ.get('INTEGRATION') is None, reason="only ran with integration VMs")
@pytest.mark.asyncio
async def test_scan_by_ip():
    ipv4_target = Target(ip_address("192.168.56.3"))
    builder = Builder()
    builder.set_ping(False)
    builder.set_port_list([80, 1337])
    builder.add_target(ipv4_target)
    builder.set_tracing(True)
    scan = Scan(builder)
    result = await scan.next()
    assert type(await scan.next()) is ScanFinished
    assert result.contents.ports.get(80).status == PortStatus.OPEN
    assert result.contents.ports.get(1337).status == PortStatus.CLOSED
    assert result.contents.ports.get(123) is None
