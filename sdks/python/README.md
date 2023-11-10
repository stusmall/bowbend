# How to use address sanitizer with the module:

First build it with: 
`cargo xtask build --asan`

Then you can run it in a test container with:
`docker run --rm -v $(pwd):/bowbend --net test-network python:3.10 sh -c 'ls -d /bowbend/* | grep "bowbend.*linux.*whl"  | xargs  pip3 install && apt-get update && apt-get install -y libasan6 && PYTHONMALLOC=malloc LD_PRELOAD=/usr/lib/x86_64-linux-gnu/libasan.so.6 python3 /bowbend/integration/python/integration_test.py`

# How to use GDB to debug the module:
- Create a new venv `python3 -m venv venv; source venv/bin/activate`
- Install python3 with debug symbols `sudo apt-get install python3-dbg`
- Create a link to the system python3-dbg in the venv.  `ln -s /usr/bin/python3-dbg venv/bin/activate/python3-dbg`
- Run gdb: `gdb --args python3-dbg integration/integration_test.py`
- Alternately to run gbd + asan: `gdb -ex "set exec-wrapper env LD_PRELOAD=/usr/lib/x86_64-linux-gnu/libasan.so.6" -ex "set confirm off" -ex "add-symbol-file target/x86_64-unknown-linux-gnu/debug/libbowbend.so" -ex "b new_builder" --args python3-dbg integration/integration_test.py`