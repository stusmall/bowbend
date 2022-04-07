from portscanner import PortscanBuilder, start_scan

def test_start_scan():
    builder = PortscanBuilder()
    start_scan(builder)