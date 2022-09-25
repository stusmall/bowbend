import logging
from typing import Union, Any
from janus import Queue
from .builder import Builder
from .error import Error
from .bowbend import ffi, lib  # type: ignore # noqa # pylint: disable=import-error
from .report import Report

logger = logging.getLogger(__name__)


class ScanFinished:
    logger.debug("Scan is finished")


class Scan:
    _queue: Queue[Union[Error, ScanFinished, Report]]
    _inner: Any
    _callback: Any

    def __init__(self, builder: Builder) -> None:
        @ffi.callback("void(*)(StreamItem_FfiResult_Report_t)")
        def callback(item) -> None:
            logger.debug("In callback")
            item = ffi.gc(item, lib.free_stream_item)
            if item.complete:
                self._queue.sync_q.put(ScanFinished())
            else:
                if item.item.status_code == 0:
                    report = Report(item.item.contents)
                    self._queue.sync_q.put(report)
                else:
                    error = Error(item.item.status_code)
                    self._queue.sync_q.put(error)
        # This is a bit awkward but gets the job done.  We want a callback
        # that captures a reference to the queue, doesn't have a self argument
        # and won't get GC'd before the scan is finished
        self._callback = callback
        self._queue = Queue()
        self._inner = ffi.gc(lib.start_scan(builder._inner, self._callback),
                             lib.free_scan)

    async def next(self) -> Union[Error, ScanFinished, Report]:
        return await self._queue.async_q.get()
