from ipaddress import ip_address, ip_network

from portscanner import PortscanTarget


def test_build_ipv4_target():
    ipv4_target = PortscanTarget(ip_address("192.168.0.1"))
    assert str(ipv4_target) == "192.168.0.1"


def test_build_ip4_network_target():
    network_v4 = PortscanTarget(ip_network("192.168.0.0/24"))
    assert str(network_v4) == "192.168.0.0/24"


def test_build_ipv6_target():
    ipv6_target = PortscanTarget(ip_address("1:203:405:607:809:a0b:c0d:e0f"))
    assert str(ipv6_target) == "1:203:405:607:809:a0b:c0d:e0f"


def test_build_ipv6_network_target():
    network_v6 = PortscanTarget(ip_network("::1/128"))
    assert str(network_v6) == "::1/128"


def test_build_hostname_target():
    hostname = PortscanTarget("stuartsmall.com")
    assert str(hostname) == "stuartsmall.com"
    # We want to accept even obviously invalid hostnames
    hostname = PortscanTarget("")
    assert str(hostname) == ""
