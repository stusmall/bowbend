import logging
from typing import Union
from ipaddress import IPv4Address, IPv6Address, IPv4Network, IPv6Network
from _cffi_backend import _CDataBase  # type: ignore

from ._utils import _char_star_to_python_string, FfiByteArray
from .bowbend import ffi, lib  # type: ignore # noqa # pylint: disable=import-error

logger = logging.getLogger(__name__)


class Target:
    """
    This is a potential target for a port scan.  It can a hostname, an IP
    address or a network of IP address.  Initialize it with a string to have
    it treated as a hostname.  Use an IPv4Address or IPv6Address instance for
    individual IP addresses.  Use IPv4Network or IPv6Network for networks.
    Additionally, this can be initialized with a cffi object but that is for
    internal use only.
    """
    _inner: _CDataBase

    def __init__(self, target: Union[IPv4Address, IPv6Address, IPv4Network,
                                     IPv6Network, str, _CDataBase]) -> None:
        if isinstance(target, IPv4Address):
            address = FfiByteArray(target.packed)
            result = lib.new_ip_v4_address(address.get_slice())
        elif isinstance(target, IPv6Address):
            address = FfiByteArray(target.packed)
            result = lib.new_ip_v6_address(address.get_slice())
        elif isinstance(target, IPv4Network):
            arg1 = FfiByteArray(target.network_address.packed)
            arg2 = target.prefixlen
            result = lib.new_ip_v4_network(arg1.get_slice(), arg2)
        elif isinstance(target, IPv6Network):
            arg1 = FfiByteArray(target.network_address.packed)
            arg2 = target.prefixlen
            result = lib.new_ip_v6_network(arg1.get_slice(), arg2)
        elif isinstance(target, str):
            hostname = FfiByteArray(target.encode("UTF-8"))
            result = lib.new_hostname(hostname.get_slice())
        elif isinstance(target, _CDataBase):
            self._inner = target
            return
        else:
            raise ValueError("Not a valid type of target")

        self._inner = ffi.gc(result, lib.free_target)

        if result.status_code == lib.STATUS_CODES_OK:
            pass
        else:
            raise ValueError("Failed to build an target")

    def __str__(self) -> str:
        c_str = lib.display_target(self._inner.contents)
        return _char_star_to_python_string(c_str)
