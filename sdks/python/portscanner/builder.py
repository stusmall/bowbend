from typing import Any, List
from .portscanner import lib  # type: ignore
from .targets import PortscanTarget


class PortscanBuilder:
    __inner: Any

    def __init__(self) -> None:
        self.__inner = lib.new_portscan_builder()

    def add_target(self, target: PortscanTarget) -> None:
        lib.add_target(self.__inner, target)

    def set_port_list(self, ports: List[int]) -> None:
        lib.set_port_list(self.__inner, ports)
