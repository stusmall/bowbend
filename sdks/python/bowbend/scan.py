from . import Builder
from .bowbend import ffi, lib  # type: ignore


def start_scan(builder: Builder):
    @ffi.callback("void(*)()")
    def callback():
        print("In callback")

    lib.start_scan(builder._inner, callback)
