from enum import Enum


class Error(Enum):
    INVALID_LENGTH = -1
    INVALID_UTF8 = -2
    FAILED_TO_RESOLVE_HOSTNAME = -3
    INSUFFICIENT_PERMISSION = -4
    UNKNOWN_ERROR = -100
