from typing import Any

from .bowbend import ffi  # type: ignore # noqa # pylint: disable=import-error


class FfiByteArray:
    def __init__(self, byte_array: bytes):
        self._slice = ffi.new("slice_ref_uint8_t*")
        self._buffer = ffi.new("uint8_t []", len(byte_array))
        self._slice[0].ptr = self._buffer
        for index, byte in enumerate(byte_array):
            self._slice[0].ptr[index] = byte
        self._slice[0].len = len(byte_array)

    def get_slice(self):
        return self._slice[0]


def _char_star_to_python_string(ffi_string: Any) -> str:
    return ffi.string(ffi_string).decode('UTF-8')


def _vec_uint8_to_python_string(ffi_string: Any) -> str:
    return ffi.string(ffi_string.ptr, maxlen=ffi_string.len).decode('UTF-8')
