use std::{
    io::{BufRead, BufReader, Read, Write},
    net::TcpStream,
    process::{Child, Command, Stdio},
    sync::mpsc,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
struct Engine(Child);
impl Drop for Engine {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}
fn request(address: &str, path: &str, body: Option<&str>) -> serde_json::Value {
    let mut socket = TcpStream::connect(address).unwrap();
    socket
        .set_read_timeout(Some(Duration::from_secs(5)))
        .unwrap();
    let body = body.unwrap_or("");
    write!(socket,"{} {} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",if body.is_empty(){"GET"}else{"POST"},path,address,body.len(),body).unwrap();
    let mut bytes = Vec::new();
    let mut chunk = [0u8; 8192];
    loop {
        let count = socket.read(&mut chunk).unwrap();
        assert!(count > 0, "HTTP response ended before its declared body");
        bytes.extend_from_slice(&chunk[..count]);
        if let Some(end) = bytes.windows(4).position(|part| part == b"\r\n\r\n") {
            let header = std::str::from_utf8(&bytes[..end]).unwrap();
            let length = header
                .lines()
                .find_map(|line| {
                    let (key, value) = line.split_once(':')?;
                    key.eq_ignore_ascii_case("content-length")
                        .then(|| value.trim().parse::<usize>().unwrap())
                })
                .expect("JSON responses have a known content length");
            if bytes.len() >= end + 4 + length {
                break;
            }
        }
    }
    let response = String::from_utf8(bytes).unwrap();
    assert!(response.starts_with("HTTP/1.1 200"), "{response}");
    serde_json::from_str(response.split_once("\r\n\r\n").unwrap().1).unwrap()
}
#[test]
fn managed_shutdown_flushes_the_latest_project_and_history() {
    let root = std::env::temp_dir().join(format!(
        "schema-managed-stop-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let stop = root.join("shutdown.request");
    let mut engine = Engine(
        Command::new(env!("CARGO_BIN_EXE_schema-rs"))
            .args(["serve", "--bind", "127.0.0.1:0", "--project"])
            .arg(&root)
            .arg("--shutdown-file")
            .arg(&stop)
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap(),
    );
    let stdout = engine.0.stdout.take().unwrap();
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        for line in BufReader::new(stdout).lines().map_while(Result::ok) {
            if let Some((_, address)) = line.split_once("http://") {
                let _ = tx.send(address.trim().to_owned());
                break;
            }
        }
    });
    let address = rx.recv_timeout(Duration::from_secs(10)).unwrap();
    let before = request(&address, "/api/state/compact", None);
    assert_eq!(before["project"]["output"]["enabled"], false);
    request(
        &address,
        "/api/command",
        Some(
            r#"{"expected_revision":0,"command":{"type":"set_dimmer","fixture":"fixture-1","value":0.37}}"#,
        ),
    );
    let expected = request(&address, "/api/session", None);
    std::fs::write(&stop, "stop").unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if let Some(status) = engine.0.try_wait().unwrap() {
            assert!(status.success());
            break;
        }
        assert!(
            Instant::now() < deadline,
            "managed shutdown did not stop the engine"
        );
        std::thread::sleep(Duration::from_millis(20));
    }
    let saved: serde_json::Value =
        serde_json::from_slice(&std::fs::read(root.join(".session/session.json")).unwrap())
            .unwrap();
    for key in ["project", "undo", "redo"] {
        assert_eq!(saved[key], expected[key], "{key}");
    }
    assert_eq!(saved["snapshot"]["revision"], 1);
    assert!(!saved["undo"].as_array().unwrap().is_empty());
    std::fs::remove_dir_all(&root).unwrap();
}
