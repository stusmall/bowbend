from . import PortscanBuilder
from .portscanner import ffi, lib  # type: ignore


def start_scan(builder: PortscanBuilder):
    @ffi.callback("void(*)()")
    def callback():
        print("In callback")

    lib.start_scan(builder._inner, callback)
