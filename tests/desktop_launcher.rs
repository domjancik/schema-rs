use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    process::{Command, Stdio},
    time::{Duration, Instant},
};
struct Child(std::process::Child);
impl Drop for Child {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}
#[test]
fn desktop_starts_an_isolated_loopback_engine_with_output_disarmed() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    drop(listener);
    let root = std::env::temp_dir().join(format!("schema-desktop-{}", std::process::id()));
    let mut child = Child(
        Command::new(env!("CARGO_BIN_EXE_schema-rs"))
            .args([
                "desktop",
                "--no-open",
                "--project",
                root.to_str().unwrap(),
                "--bind",
                &address.to_string(),
            ])
            .env_remove("SCHEMA_TOKEN")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap(),
    );
    let deadline = Instant::now() + Duration::from_secs(10);
    let mut stream = loop {
        if let Ok(stream) = TcpStream::connect(address) {
            break stream;
        }
        assert!(
            child.0.try_wait().unwrap().is_none(),
            "desktop command exited without serving"
        );
        assert!(Instant::now() < deadline, "desktop failed to listen");
        std::thread::sleep(Duration::from_millis(20));
    };
    stream
        .set_read_timeout(Some(Duration::from_secs(3)))
        .unwrap();
    write!(
        stream,
        "GET /api/state HTTP/1.1\r\nHost: {address}\r\nConnection: close\r\n\r\n"
    )
    .unwrap();
    let mut response = String::new();
    stream.read_to_string(&mut response).unwrap();
    assert!(response.starts_with("HTTP/1.1 200"));
    let state: serde_json::Value =
        serde_json::from_str(response.split_once("\r\n\r\n").unwrap().1).unwrap();
    assert_eq!(state["project"]["output"]["enabled"], false);
    drop(child);
    let _ = std::fs::remove_dir_all(root);
}
