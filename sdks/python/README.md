To build and test, from root run:
```
cargo xtask build && pip install --force-reinstall /home/stusmall/Workspace/portscanner/sdks/python/target/wheels/portscanner-0.1.0-py3-none-linux_x86_64.whl && python sdks/python/tests/test.py
```

Currently source distributions don't work: https://github.com/PyO3/maturin/issues/831