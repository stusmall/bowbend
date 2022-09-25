#How to use address sanitizer with the module:

maturin build --rustc-extra-args="-Clink-arg=-lasan -Zsanitizer=address" --target x86_64-unknown-linux-gnu && pip install --force-reinstall target/wheels/bowbend-0.1.0-py3-none-linux_x86_64.whl && LD_PRELOAD=/usr/lib/x86_64-linux-gnu/libasan.so.6 python3 integration/integration_test.py

# How to use GDB to debug the module:
- Create a new venv `python3 -m venv venv; source venv/bin/activate`
- Install python3 with debug symbols `sudo apt-get install python3-dbg`
- Create a link to the system python3-dbg in the venv.  `ln -s /usr/bin/python3-dbg venv/bin/activate/python3-dbg`
- Run gdb: `gdb --args python3-dbg integration/integration_test.py`
- Alternately to run gbd + asan: `gdb -ex "set exec-wrapper env LD_PRELOAD=/usr/lib/x86_64-linux-gnu/libasan.so.6" -ex "set confirm off" -ex "add-symbol-file target/x86_64-unknown-linux-gnu/debug/libbowbend.so" -ex "b new_builder" --args python3-dbg integration/integration_test.py`