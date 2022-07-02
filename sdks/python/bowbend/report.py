from _cffi_backend import _CDataBase  # type: ignore

#TODO: make this usable

class Report:
    _inner: _CDataBase

    def __init__(self, internal: _CDataBase) -> None:
        self._inner = internal


    def __str__(self):
        return "idk"
