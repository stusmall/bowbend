from typing import Any, List
from .bowbend import lib  # type: ignore # noqa # pylint: disable=import-error
from .targets import Target


class Builder:
    _inner: Any

    def __init__(self) -> None:
        self._inner = lib.new_builder()

    def add_target(self, target: Target) -> None:
        lib.add_target(self._inner, target._inner)

    def set_port_list(self, ports: List[int]) -> None:
        lib.set_port_list(self._inner, ports)

    def set_ping(self, ping: bool) -> None:
        lib.set_ping(self._inner, ping)

    def set_tracing(self, tracing: bool) -> None:
        lib.set_tracing(self._inner, tracing)
