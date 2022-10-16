from enum import Enum
from typing import Optional

from _cffi_backend import _CDataBase  # type: ignore
from bowbend._utils import _vec_uint8_to_python_string

from .bowbend import ffi  # type: ignore # noqa # pylint: disable=import-error


class Certainty(Enum):
    ADVERTISED = 0
    HIGH = 1
    MEDIUM = 2
    LOW = 3

    def __str__(self) -> str:
        match self:
            case Certainty.ADVERTISED:
                return "advertised"
            case Certainty.HIGH:
                return "high"
            case Certainty.MEDIUM:
                return "medium"
            case Certainty.LOW:
                return "low"


class ServiceDetectionConclusion:
    certainty: Certainty
    service_name: str
    service_version: Optional[str]

    def __init__(self, internal: _CDataBase):
        assert ffi.typeof(internal) is \
               ffi.typeof("struct ServiceDetectionConclusion")
        self.certainty = Certainty(internal.certainty)
        self.service_name = _vec_uint8_to_python_string(internal.service_name)

        if internal.service_version != ffi.NULL:
            self.service_version = \
                _vec_uint8_to_python_string(internal.service_version)
        else:
            self.service_version = None

    def __str__(self) -> str:
        if self.service_version:
            return f"running {self.service_name} at version " \
                   f"{self.service_version} with {self.certainty} certainty"

        return f"running {self.service_name} with {self.certainty} certainty"
