from enum import Enum


class Error(Enum):
    InvalidLength = -1
    InvalidUTF8 = -2
    FailedToResolveHostname = -3
    InsufficientPermission = -4
    UnknownError = -5

    def __str__(self):
        raise Exception()

