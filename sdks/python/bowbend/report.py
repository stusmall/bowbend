from enum import Enum
from typing import Union, Optional, Dict, List
from ipaddress import IPv4Address, IPv6Address

from _cffi_backend import _CDataBase  # type: ignore

from .error import Error
from .bowbend import ffi  # type: ignore # noqa # pylint: disable=import-error
from .target import Target
from .service_detection import ServiceDetectionConclusion


class PortStatus(Enum):
    OPEN = 0
    CLOSED = 1

    def __str__(self) -> str:
        match self:
            case PortStatus.OPEN:
                return "open"
            case PortStatus.CLOSED:
                return "closed"
            case _:
                raise NotImplementedError


class PortReport:
    port: int
    status: PortStatus
    service_detection_conclusions: Optional[List[ServiceDetectionConclusion]]

    def __init__(self, internal):
        assert ffi.typeof(internal) is ffi.typeof("struct PortReport")
        self.port = internal.port
        self.status = PortStatus(internal.status)
        if ffi.NULL not in (internal.service_detection_conclusions,
                            internal.service_detection_conclusions.ptr):
            self.service_detection_conclusions = []
            for i in range(internal.service_detection_conclusions.len):
                entry = ServiceDetectionConclusion(
                    internal.service_detection_conclusions.ptr[i])
                self.service_detection_conclusions.append(entry)
        else:
            self.service_detection_conclusions = None

    def __str__(self):
        if self.service_detection_conclusions is None:
            return f"Port {self.port} is {self.status}"
        to_ret = f"Port {self.port} is {self.status}. Service detection(s):\n"
        for service_detection in self.service_detection_conclusions:
            to_ret = to_ret + "\t" + str(service_detection) + "\n"
        return to_ret


class PingResultType(Enum):
    RECEIVED_REPLY = 0
    IO_ERROR = 1
    TIMEOUT = 2


class PingResult:
    ping_result_type: PingResultType

    def __init__(self, result_type: int):
        self.ping_result_type = PingResultType(result_type)


class ReportContents:
    ping_result: Optional[PingResult]
    ports: Dict[int, PortReport]

    def __init__(self, internal: _CDataBase):
        assert ffi.typeof(internal) is ffi.typeof("ReportContents_t*")
        if internal.icmp != ffi.NULL:
            self.ping_result = PingResult(internal.icmp.result_type)

        self.ports = {}
        for i in range(internal.ports.len):
            # We are going to flatten this out a bit.  We don't have a great
            # way to pass a Dict over the FFI layer, so we are just passing
            # over a vec of reports and leaving it up to each SDK to turn it
            # into a dictionary
            port = internal.ports.ptr[i].port
            report = PortReport(internal.ports.ptr[i])
            self.ports[port] = report

    def __str__(self):
        to_ret = ""
        for port in self.ports.items():
            if port[1] is not None:
                to_ret = to_ret + "\n" + str(port[1])
        return to_ret


class Report:
    target: Target
    instance: Optional[Union[IPv4Address, IPv6Address]]

    contents: Union[ReportContents, Error]

    def __init__(self, internal: _CDataBase) -> None:
        assert ffi.typeof(internal) is ffi.typeof("Report_t*")
        self.target = Target(ffi.addressof(internal.target))
        if internal.instance.ip:
            internal_ip = internal.instance.ip

            ip_bytes = bytes(ffi.buffer(internal_ip.ptr, internal_ip.len))
            if len(ip_bytes) == 4:
                self.instance = IPv4Address(ip_bytes)
            elif len(ip_bytes) == 16:
                self.instance = IPv6Address(ip_bytes)
            else:
                raise Exception("Internal failure. An IP was returned with an "
                                "invalid number of bytes")
        else:
            self.instance = None

        if internal.contents.status_code == 0:
            self.contents = ReportContents(internal.contents.contents)
        else:
            self.contents = Error(internal.contents.status_code)

    def __str__(self):
        return f"Scanned {self.instance} as part of {self.target}.  The " \
               f"results are: " + str(self.contents)
