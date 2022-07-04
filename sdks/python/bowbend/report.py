from typing import Union
from ipaddress import IPv4Address, IPv6Address

from _cffi_backend import _CDataBase  # type: ignore

from ._utils import _vec_unit8_t_to_bytes
from .bowbend import ffi  # type: ignore # noqa # pylint: disable=import-error
from .target import Target


class Report:
    target: Target
    instance: Union[IPv4Address, IPv6Address]

    def __init__(self, internal: _CDataBase) -> None:
        self.target = Target(ffi.addressof(internal.target))
        print("Building instance...")
        print(f"Type of instance {_vec_unit8_t_to_bytes(internal.instance.ip)}")

    def __str__(self):
        return "idk"
