from threading import Thread
from typing import Union
from janus import AsyncQueue, SyncQueue, Queue
from .builder import Builder
from .error import Error
from .bowbend import ffi, lib  # type: ignore # noqa # pylint: disable=import-error
from .report import Report


class ScanFinished:
    def __init__(self):
        print("Scan is finished")


def _spawn_scan(builder: Builder,
                queue: SyncQueue[Union[Error, ScanFinished, Report]]):
    @ffi.callback("void(*)(StreamItem_FfiResult_Report_t)")
    def callback(item) -> None:
        if item.complete:
            queue.put(ScanFinished())
        else:
            if item.item.status_code == 0:
                report = Report(item.item.contents)
                queue.put(report)
            else:
                error = Error(item.item.status_code)
                queue.put(error)
    lib.start_scan(builder._inner, callback)


class Scan:
    _queue: AsyncQueue[Union[Error, ScanFinished, Report]]
    _thread: Thread

    def __init__(self, builder: Builder) -> None:
        queue: Queue = Queue()
        self._queue = queue.async_q
        print("Starting thread")
        self._thread = Thread(target=_spawn_scan,
                              args=(builder, queue.sync_q,))
        self._thread.start()

    async def next(self) -> Union[Error, ScanFinished, Report]:
        return await self._queue.get()
