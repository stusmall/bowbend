from typing import Any

from _cffi_backend import _CDataBase  # type: ignore

from .bowbend import ffi  # type: ignore # noqa # pylint: disable=import-error


def _bytes_to_slice_ref_unit8_t(byte_array: bytes) -> _CDataBase:
    slice_ref = ffi.new("slice_ref_uint8_t[]", 1)
    slice_ref[0].ptr = ffi.new("uint8_t const []", len(byte_array))
    for index, byte in enumerate(byte_array):
        slice_ref[0].ptr[index] = byte
    slice_ref[0].len = len(byte_array)
    return slice_ref[0]


def _char_star_to_python_string(ffi_string: Any) -> str:
    return ffi.string(ffi_string).decode('UTF-8')
