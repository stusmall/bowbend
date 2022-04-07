from typing import Any, List
from .portscanner import lib  # type: ignore
from .targets import PortscanTarget


class PortscanBuilder:
    _inner: Any

    def __init__(self) -> None:
        self._inner = lib.new_portscan_builder()

    def add_target(self, target: PortscanTarget) -> None:
        lib.add_target(self._inner, target)

    def set_port_list(self, ports: List[int]) -> None:
        lib.set_port_list(self._inner, ports)
