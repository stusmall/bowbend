from typing import Union
from ipaddress import IPv4Address, IPv6Address

from _cffi_backend import _CDataBase  # type: ignore
from .bowbend import ffi  # type: ignore # noqa # pylint: disable=import-error
from .target import Target


class Report:
    target: Target
    instance: Union[IPv4Address, IPv6Address]

    def __init__(self, internal: _CDataBase) -> None:
        self.target = Target(ffi.addressof(internal.target))

    def __str__(self):
        return "idk"
