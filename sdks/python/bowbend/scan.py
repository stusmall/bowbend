from threading import Thread
from typing import Union
from janus import Queue
from .builder import Builder
from .error import Error
from .bowbend import ffi, lib  # type: ignore # noqa # pylint: disable=import-error
from .report import Report


class ScanFinished:
    def __init__(self):
        print("Scan is finished")


def _spawn_scan(builder: Builder,
                queue: Queue[Union[Error, ScanFinished, Report]]):
    @ffi.callback("void(*)(FfiResult_Report_t)")
    def callback(report_result) -> None:
        print(f"PYTHON: In callback {report_result}")
        if report_result.status_code == 0:
            report = Report(report_result.contents)
            queue.put(report)
        else:
            error = Error(report_result.status_code)
            print("We have an error")
            queue.put(error)
    lib.start_scan(builder._inner, callback)
    print("Adding scan finished to queue")
    queue.put(ScanFinished())


class Scan:
    _queue: Queue[Union[Error, ScanFinished, Report]]
    _thread: Thread

    def __init__(self, builder: Builder) -> None:
        self._queue = Queue()
        print("Starting thread")
        self._thread = Thread(target=_spawn_scan,
                              args=(builder, self._queue.sync_q,))
        self._thread.start()

    async def next(self) -> Union[Error, ScanFinished, Report]:
        return await self._queue.async_q.get()
