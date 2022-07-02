from typing import Union
from ipaddress import IPv4Address, IPv6Address, IPv4Network, IPv6Network
from _cffi_backend import _CDataBase  # type: ignore

from ._utils import _bytes_to_slice_ref_unit8_t, _char_star_to_python_string
from .bowbend import lib  # type: ignore # noqa # pylint: disable=import-error


class Target:
    """
    This is a potential target for a port scan.  It can a hostname, an IP address or a network of IP address.
    Initialize it with a string to have it treated as a hostname.  Use an IPv4Address or IPv6Address instance for
    individual IP addresses.  Use IPv4Network or IPv6Network for networks.  Additionally, this can be initialized with
    a cffi object but that is for internal use only.
    """
    _inner: _CDataBase

    def __init__(self, target: Union[IPv4Address, IPv6Address, IPv4Network,
                                     IPv6Network, str, _CDataBase]) -> None:
        if isinstance(target, IPv4Address):
            address = _bytes_to_slice_ref_unit8_t(target.packed)
            result = lib.new_ip_v4_address(address)
        elif isinstance(target, IPv6Address):
            address = _bytes_to_slice_ref_unit8_t(target.packed)
            result = lib.new_ip_v6_address(address)
        elif isinstance(target, IPv4Network):
            arg1 = _bytes_to_slice_ref_unit8_t(target.network_address.packed)
            arg2 = target.prefixlen
            result = lib.new_ip_v4_network(arg1, arg2)
        elif isinstance(target, IPv6Network):
            arg1 = _bytes_to_slice_ref_unit8_t(target.network_address.packed)
            arg2 = target.prefixlen
            result = lib.new_ip_v6_network(arg1, arg2)
        elif isinstance(target, str):
            arg1 = _bytes_to_slice_ref_unit8_t(target.encode("UTF-8"))
            result = lib.new_hostname(arg1)
        elif isinstance(target, _CDataBase):
            self._inner = target
            return
        else:
            raise ValueError("Not a valid type of target")

        if result.status_code == lib.STATUS_CODES_OK:
            self._inner = result.contents
        else:
            raise ValueError("Failed to build an target")

    def __str__(self) -> str:
        c_str = lib.display_target(self._inner)
        return _char_star_to_python_string(c_str)
