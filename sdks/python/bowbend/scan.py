import asyncio
import janus
from . import Builder
from .bowbend import ffi, lib  # type: ignore


# Something here is corrupting memory causing the first test after it to always fail
async def start_scan(builder: Builder):
    def spawn_scan(builder: Builder, queue: janus.SyncQueue[int]):
        @ffi.callback("void(*)(Report_t)")
        def callback(report) -> None:
            print(f"In callback {report}")
            queue.put(1)

        lib.start_scan(builder._inner, callback)

    queue: janus.Queue[int] = janus.Queue()
    loop = asyncio.get_running_loop()
    fut = loop.run_in_executor(None, spawn_scan, builder, queue.sync_q) #TODO: Do I want this or to use a thread?  what happens if I don't await the fut?
    print("Waiting on fut")
    await fut
    print("Waiting on report")
    report = await queue.async_q.get()
    print("After report {}", report)
