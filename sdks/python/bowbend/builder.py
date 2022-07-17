from typing import Any, List
from .bowbend import ffi, lib  # type: ignore # noqa # pylint: disable=import-error
from .target import Target


class Builder:
    _inner: Any

    def __init__(self) -> None:
        self._inner = lib.new_builder()

    def add_target(self, target: Target) -> None:
        lib.add_target(self._inner, target._inner)

    def set_port_list(self, ports: List[int]) -> None:
        slice_ref = ffi.new("slice_ref_uint16_t[]", 1)

        slice_ref[0].ptr = ffi.new("uint16_t const []", len(ports))
        for index, port in enumerate(ports):
            slice_ref[0].ptr[index] = port
        slice_ref[0].len = len(ports)
        lib.set_port_list(self._inner, slice_ref[0])

    def set_ping(self, ping: bool) -> None:
        lib.set_ping(self._inner, ping)

    def set_tracing(self, tracing: bool) -> None:
        lib.set_tracing(self._inner, tracing)
