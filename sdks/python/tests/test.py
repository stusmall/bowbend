from ipaddress import IPv4Network, IPv4Address, ip_address, ip_network

from portscanner import PortscanBuilder, PortscanTarget

import gc

builder = PortscanBuilder()


# ipv4 = PortscanTarget(ip_address("192.168.0.1"))
# print("Contents of ipv4 " + str(ipv4))
# networkv4 = PortscanTarget(ip_network("192.168.0.0/24"))
# print("Contents of networkv4 is " + str(networkv4))


PortscanTarget(ip_address("1:203:405:607:809:a0b:c0d:e0f"))
# print("Contents of ipv6 " + str(ipv6))
# networkv6 = PortscanTarget(ip_network("::1/128"))
# print("Contents of networkv6 is " + str(networkv6))
#
#
# hostname = PortscanTarget("stuartsmall.com")
# print("Contents of hostname " + str(hostname))
print("exiting")
