import asyncio
import janus
from .builder import Builder
from .bowbend import ffi, lib  # type: ignore # noqa # pylint: disable=import-error
from .report import Report


async def start_scan(builder: Builder):
    def spawn_scan(builder: Builder, queue: janus.SyncQueue[Report]):
        @ffi.callback("void(*)(FfiResult_Report_t)")
        def callback(report_result) -> None:
            print(f"In callback {report_result}")
            # TODO: Properly check the result
            report = Report(report_result.contents)
            queue.put(report)

        lib.start_scan(builder._inner, callback)

    queue: janus.Queue[Report] = janus.Queue()
    loop = asyncio.get_running_loop()
    # TODO: Do I want this or to use a thread?  what happens if I don't
    #  await the fut?
    fut = loop.run_in_executor(None, spawn_scan, builder, queue.sync_q)
    print("Waiting on fut")
    await fut
    print("Waiting on report")
    report = await queue.async_q.get()
    print("After report {}", report)
