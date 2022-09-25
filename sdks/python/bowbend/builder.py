import logging
from typing import Any, List
from .bowbend import ffi, lib  # type: ignore # noqa # pylint: disable=import-error
from .target import Target

logger = logging.getLogger(__name__)


class Builder:
    _inner: Any

    def __init__(self) -> None:
        self._inner = ffi.gc(lib.new_builder(), lib.free_builder)

    def add_target(self, target: Target) -> None:
        # logger.debug(f"Adding target {target}")
        lib.add_target(self._inner, target._inner.contents)

    def set_port_list(self, ports: List[int]) -> None:
        logger.debug("Setting port list %s", ports)
        slice_ref = ffi.new("slice_ref_uint16_t[]", 1)
        ptr = ffi.new("uint16_t const []", len(ports))
        slice_ref[0].ptr = ptr
        for index, port in enumerate(ports):
            slice_ref[0].ptr[index] = port
        slice_ref[0].len = len(ports)
        lib.set_port_list(self._inner, slice_ref[0])

    def set_ping(self, ping: bool) -> None:
        lib.set_ping(self._inner, ping)

    def set_tracing(self, tracing: bool) -> None:
        lib.set_tracing(self._inner, tracing)
