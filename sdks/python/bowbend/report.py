from _cffi_backend import _CDataBase  # type: ignore


class Report:
    _inner: _CDataBase

    def __init__(self, internal: _CDataBase) -> None:
        self._inner = internal
        print("Creating target with  " + str(type(internal.target)))
        print("Checking if instance of " +
              str(isinstance(internal.target, _CDataBase)))
