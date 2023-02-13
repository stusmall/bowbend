import asyncio
import logging
from ipaddress import ip_address
from typing import Union

from bowbend import Builder, Scan, Target, Error

from bowbend.report import PortStatus, Report, PingResultType, ReportContents
from bowbend.scan import ScanFinished


async def basic_ip_scan():
    ipv4_target = Target(ip_address("192.168.56.3"))
    builder = Builder()
    builder.set_ping(False)
    builder.set_port_list([80, 1337])
    builder.add_target(ipv4_target)
    scan = Scan(builder)
    result = await scan.next()
    assert type(await scan.next()) is ScanFinished
    assert result.contents.ports.get(80).status == PortStatus.OPEN
    assert result.contents.ports.get(1337).status == PortStatus.CLOSED
    assert result.contents.ports.get(123) is None
    print("Basic scan test passed")


async def scan_with_icmp():
    def assert_result(result: Union[Error, ScanFinished, Report]):
        if isinstance(result, Error):
            raise Exception("unexpected error")
        elif isinstance(result, ScanFinished):
            raise Exception("Stream finished too soon")
        elif isinstance(result, Report):
            assert (isinstance(result.contents, ReportContents))
            assert (result.contents.ping_result is not None)
            if result.instance == ip_address("192.168.56.3"):
                assert (result.contents.ping_result.ping_result_type == PingResultType.RECEIVED_REPLY)
            elif result.instance == ip_address("192.168.56.4"):
                assert (result.contents.ping_result.ping_result_type == PingResultType.TIMEOUT)
            else:
                raise Exception("This doesn't match either target")
        else:
            raise Exception("Unexpected result")

    builder = Builder()
    builder.set_ping(True)
    builder.set_port_list([22])
    builder.add_target(Target(ip_address("192.168.56.3")))
    builder.add_target(Target(ip_address("192.168.56.4")))
    scan = Scan(builder)
    assert_result(await scan.next())
    assert_result(await scan.next())
    assert type(await scan.next()) is ScanFinished
    print("ICMP scan test passed")


async def scan_with_service_detection():
    builder = Builder()
    builder.set_run_service_detection(True)
    builder.set_port_list([80])
    builder.add_target(Target("web"))
    scan = Scan(builder)
    result = await scan.next()
    assert "nginx" in result.contents.ports[80].service_detection_conclusions[0].service_name

    assert type(await scan.next()) is ScanFinished
    print("Scan with service detection passed")


async def main():
    logging.basicConfig(level=logging.DEBUG)
    await basic_ip_scan()
    await scan_with_icmp()
    await scan_with_service_detection()


if __name__ == "__main__":
    asyncio.run(main())
