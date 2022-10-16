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

    def set_run_service_detection(self, run_service_detection: bool) -> None:
        lib.set_run_service_detection(self._inner, run_service_detection)

    def set_ping(self, ping: bool) -> None:
        lib.set_ping(self._inner, ping)

    def set_tracing(self, tracing: bool) -> None:
        logger.debug("Setting tracing %r", tracing)
        lib.set_tracing(self._inner, tracing)

    def set_throttle(self, minimum: int, maximum: int) -> None:
        """ Set a range for random pauses to be inserted in various points
        during a scan.  Each time this is applied a random value between
        minimum and maximum will be used. """
        result = lib.set_throttle(self._inner, minimum, maximum)
        if result.status_code != lib.STATUS_CODES_OK:
            raise ValueError("Failed to set throttle")

    def set_max_in_flight(self, max_in_flight: int) -> None:
        """ Set the maximum number of in flight tasks for a port scan.  This
        is useful for limiting resource utilization. """
        logger.debug("Setting max in flight %s", max_in_flight)
        lib.set_max_in_flight(self._inner, max_in_flight)
