use schema_rs::{artnet, model::Project, server::Runtime};
#[test]
fn artdmx_sender_transmits_valid_loopback_datagram() {
    let receiver = std::net::UdpSocket::bind("127.0.0.1:0").unwrap();
    receiver
        .set_read_timeout(Some(std::time::Duration::from_secs(1)))
        .unwrap();
    let mut sender = artnet::Sender::new().unwrap();
    sender
        .send(
            receiver.local_addr().unwrap(),
            &[(7, vec![255, 128, 0])].into(),
        )
        .unwrap();
    let mut data = [0; 1024];
    let (n, _) = receiver.recv_from(&mut data).unwrap();
    assert_eq!(&data[..8], b"Art-Net\0");
    assert_eq!(n, 22);
    assert_eq!(&data[14..18], &[7, 0, 0, 4]);
    assert_eq!(&data[18..22], &[255, 128, 0, 0]);
}
#[test]
fn engine_output_worker_sends_to_explicit_loopback_target() {
    let receiver = std::net::UdpSocket::bind("127.0.0.1:0").unwrap();
    receiver
        .set_read_timeout(Some(std::time::Duration::from_secs(1)))
        .unwrap();
    let mut p = Project::demo();
    p.output.enabled = true;
    p.output.target = receiver.local_addr().unwrap().to_string();
    let _runtime = Runtime::start(p).unwrap();
    let mut data = [0; 1024];
    let (n, _) = receiver.recv_from(&mut data).unwrap();
    assert_eq!(n, 530);
    assert_eq!(&data[..8], b"Art-Net\0");
}
