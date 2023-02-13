use std::net::IpAddr;

use bowbend::{
    start_scan, ConfigBuilder, PingResultType, PortStatus, Report, Target, TargetInstance,
};
use futures_util::stream::StreamExt;

async fn basic_ip_scan() {
    let target: Target = Target::IP("192.168.56.3".parse::<IpAddr>().unwrap());
    let mut builder = ConfigBuilder::default();
    builder.set_ping(false);
    builder.set_port_list(vec![80, 1337]);
    builder.set_max_in_flight(10);
    builder.add_target(target);
    let stream = start_scan(builder).await.unwrap();
    let mut reports = stream.collect::<Vec<Report>>().await;
    assert_eq!(reports.len(), 1);
    let report = reports.pop().unwrap();
    let ports = report.contents.unwrap().ports.unwrap();
    assert_eq!(ports.get(&80).unwrap().status, PortStatus::Open);
    assert_eq!(ports.get(&1337).unwrap().status, PortStatus::Closed);
    assert!(ports.get(&123).is_none());
    println!("Basic scan test passed");
}

async fn scan_with_icmp() {
    fn assert_results(report: Report) {
        match report.instance.unwrap() {
            TargetInstance::IP(x) => match x.to_string().as_str() {
                "192.168.56.3" => {
                    assert!(matches!(
                        report.contents.unwrap().icmp.unwrap().result_type,
                        PingResultType::Reply(_)
                    ));
                }
                "192.168.56.4" => {
                    assert!(matches!(
                        report.contents.unwrap().icmp.unwrap().result_type,
                        PingResultType::Timeout
                    ));
                }
                _ => panic!("This doesn't match either target"),
            },
            _ => panic!("Unexpected result"),
        }
    }

    let mut builder = ConfigBuilder::default();
    builder.set_ping(true);
    builder.set_port_list(vec![80, 1337]);
    builder.add_target(Target::IP("192.168.56.3".parse::<IpAddr>().unwrap()));
    builder.add_target(Target::IP("192.168.56.4".parse::<IpAddr>().unwrap()));
    let stream = start_scan(builder).await.unwrap();
    let mut reports = stream.collect::<Vec<Report>>().await;
    assert_eq!(reports.len(), 2);
    assert_results(reports.pop().unwrap());
    assert_results(reports.pop().unwrap());
    println!("ICMP scan test passed");
}

async fn scan_with_service_detection() {
    let mut builder = ConfigBuilder::default();
    builder.set_run_service_detection(true);
    builder.set_port_list(vec![80]);
    builder.add_target(Target::Hostname("web".to_string()));
    let stream = start_scan(builder).await.unwrap();
    let mut reports = stream.collect::<Vec<Report>>().await;
    assert_eq!(reports.len(), 1);
    let report = reports.pop().unwrap();
    let ports = report.contents.unwrap().ports.unwrap();
    assert!(ports
        .get(&80)
        .unwrap()
        .service_detection_conclusions
        .clone()
        .unwrap()
        .first()
        .unwrap()
        .service_name
        .contains("nginx"));
    println!("Scan with service detection passed");
}

#[tokio::main]
async fn main() {
    basic_ip_scan().await;
    scan_with_icmp().await;
    scan_with_service_detection().await;
}
