//! Bounded acquisition adapters. File, network and analysis work never runs on the evaluator.
use crate::{
    inputs::{Audio, Event},
    model::Result,
    server::Controller,
};
use std::{
    collections::BTreeMap,
    path::{Component, Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{Duration, SystemTime, UNIX_EPOCH},
};
pub fn resource_path(root: &Path, name: &str) -> Result<PathBuf> {
    if name.is_empty()
        || name.len() > 200
        || name.contains(':')
        || name.contains('\\')
        || Path::new(name)
            .components()
            .any(|c| !matches!(c, Component::Normal(_)))
    {
        return Err("Resource must be a relative project resource path".into());
    }
    let base = root
        .join("resources")
        .canonicalize()
        .map_err(|e| e.to_string())?;
    let path = base.join(name).canonicalize().map_err(|e| e.to_string())?;
    if !path.starts_with(base) {
        return Err("Resource escapes project resources".into());
    }
    Ok(path)
}
fn string(packet: &[u8], at: &mut usize) -> Result<String> {
    let end = packet
        .get(*at..)
        .and_then(|s| s.iter().position(|v| *v == 0))
        .ok_or("Unterminated OSC string")?
        + *at;
    let text = std::str::from_utf8(&packet[*at..end])
        .map_err(|_| "Invalid OSC UTF-8")?
        .to_owned();
    *at = (end + 4) & !3;
    if *at > packet.len() {
        return Err("Truncated OSC padding".into());
    }
    Ok(text)
}
pub fn decode_osc(packet: &[u8]) -> Result<Event> {
    if packet.len() > 65507 {
        return Err("OSC packet too large".into());
    }
    let mut at = 0;
    let address = string(packet, &mut at)?;
    if !address.starts_with('/') || address.len() > 200 {
        return Err("Expected OSC address".into());
    }
    let types = string(packet, &mut at)?;
    if !types.starts_with(',') || types.len() > 65 {
        return Err("Invalid OSC type tag".into());
    }
    let mut arguments = vec![];
    for kind in types[1..].chars() {
        let value = match kind {
            'i' | 'f' => {
                let data: [u8; 4] = packet
                    .get(at..at + 4)
                    .ok_or("Truncated OSC number")?
                    .try_into()
                    .unwrap();
                at += 4;
                if kind == 'i' {
                    serde_json::json!(i32::from_be_bytes(data))
                } else {
                    let v = f32::from_be_bytes(data);
                    if !v.is_finite() {
                        return Err("Non-finite OSC number".into());
                    }
                    serde_json::json!(v)
                }
            }
            'd' | 'h' => {
                let data: [u8; 8] = packet
                    .get(at..at + 8)
                    .ok_or("Truncated OSC number")?
                    .try_into()
                    .unwrap();
                at += 8;
                if kind == 'h' {
                    serde_json::json!(i64::from_be_bytes(data))
                } else {
                    let v = f64::from_be_bytes(data);
                    if !v.is_finite() {
                        return Err("Non-finite OSC number".into());
                    }
                    serde_json::json!(v)
                }
            }
            's' => serde_json::json!(string(packet, &mut at)?),
            'T' => serde_json::json!(true),
            'F' => serde_json::json!(false),
            'N' => serde_json::Value::Null,
            _ => return Err(format!("Unsupported OSC type {kind}")),
        };
        arguments.push(value);
    }
    Ok(Event::Osc { address, arguments })
}
pub fn decode_wav(bytes: &[u8]) -> Result<Audio> {
    if bytes.len() > 16 * 1024 * 1024
        || bytes.get(..4) != Some(b"RIFF")
        || bytes.get(8..12) != Some(b"WAVE")
    {
        return Err("Expected RIFF/WAVE up to 16 MiB".into());
    }
    let mut at = 12;
    let mut format = None;
    let mut data = None;
    while at + 8 <= bytes.len() {
        let id = &bytes[at..at + 4];
        let size = u32::from_le_bytes(bytes[at + 4..at + 8].try_into().unwrap()) as usize;
        at += 8;
        let chunk = bytes.get(at..at + size).ok_or("Truncated WAV chunk")?;
        if id == b"fmt " && size >= 16 {
            format = Some((
                u16::from_le_bytes(chunk[..2].try_into().unwrap()),
                u16::from_le_bytes(chunk[2..4].try_into().unwrap()) as usize,
                u32::from_le_bytes(chunk[4..8].try_into().unwrap()),
                u16::from_le_bytes(chunk[14..16].try_into().unwrap()) as usize,
            ));
        }
        if id == b"data" {
            data = Some(chunk)
        }
        at += size + (size & 1);
    }
    let (encoding, channels, sample_rate, bits) = format.ok_or("WAV fmt missing")?;
    let data = data.ok_or("WAV data missing")?;
    if !(1..=8).contains(&channels)
        || !(8000..=192000).contains(&sample_rate)
        || !matches!((encoding, bits), (1, 8 | 16 | 24 | 32) | (3, 32))
    {
        return Err("Unsupported WAV encoding".into());
    }
    let step = bits / 8;
    let stride = channels * step;
    if data.len() % stride != 0 || data.len() / stride > 2_000_000 {
        return Err("Invalid or oversized WAV sample buffer".into());
    }
    let mut samples = Vec::with_capacity(data.len() / stride);
    for frame in data.chunks_exact(stride) {
        let mut sample = 0.;
        for value in frame.chunks_exact(step) {
            sample += match (encoding, bits) {
                (1, 8) => (value[0] as f64 - 128.) / 128.,
                (1, 16) => i16::from_le_bytes(value.try_into().unwrap()) as f64 / 32768.,
                (1, 24) => {
                    ((value[0] as i32 | ((value[1] as i32) << 8) | ((value[2] as i32) << 16)) << 8
                        >> 8) as f64
                        / 8388608.
                }
                (1, 32) => i32::from_le_bytes(value.try_into().unwrap()) as f64 / 2147483648.,
                (3, 32) => f32::from_le_bytes(value.try_into().unwrap()) as f64,
                _ => unreachable!(),
            };
        }
        sample /= channels as f64;
        if !sample.is_finite() {
            return Err("Non-finite WAV sample".into());
        }
        samples.push(sample);
    }
    Ok(Audio {
        samples,
        sample_rate,
        ..Audio::default()
    })
}
pub struct Services {
    stop: Arc<AtomicBool>,
    worker: Option<std::thread::JoinHandle<()>>,
}
impl Services {
    pub fn start(
        controller: Controller,
        root: PathBuf,
        osc: Option<std::net::SocketAddr>,
    ) -> Result<Self> {
        let socket = osc
            .map(std::net::UdpSocket::bind)
            .transpose()
            .map_err(|e| e.to_string())?;
        if let Some(socket) = &socket {
            socket.set_nonblocking(true).map_err(|e| e.to_string())?;
        }
        let stop = Arc::new(AtomicBool::new(false));
        let stopping = stop.clone();
        let worker = std::thread::Builder::new()
            .name("schema-inputs".into())
            .spawn(move || {
                let runtime = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .expect("input runtime");
                let mut scanned = std::time::Instant::now() - Duration::from_secs(2);
                let mut fingerprints = BTreeMap::<PathBuf, (u64, SystemTime)>::new();
                let mut files = BTreeMap::new();
                let mut packet = [0; 65507];
                while !stopping.load(Ordering::Relaxed) {
                    let mut events = vec![];
                    if let Some(socket) = &socket {
                        for _ in 0..32 {
                            match socket.recv_from(&mut packet) {
                                Ok((len, _)) => {
                                    if let Ok(event) = decode_osc(&packet[..len]) {
                                        events.push(event)
                                    }
                                }
                                Err(_) => break,
                            }
                        }
                    }
                    if scanned.elapsed() >= Duration::from_secs(1) {
                        scanned = std::time::Instant::now();
                        let seconds = (SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs()
                            % 86400) as u32;
                        events.push(Event::Clock { seconds });
                        let frame = controller.current();
                        let names = controller.current_plugins().resource_names(&frame.project);
                        for name in names {
                            if frame.resources.contains_key(&name) {
                                continue;
                            }
                            let path = resource_path(&root, &name);
                            let exists = path.as_ref().is_ok_and(|p| p.is_file());
                            if files.get(&name) != Some(&exists) {
                                files.insert(name.clone(), exists);
                                events.push(Event::File {
                                    path: name.clone(),
                                    exists,
                                });
                            }
                            if let Ok(path) = path {
                                if path
                                    .extension()
                                    .is_some_and(|e| e.eq_ignore_ascii_case("wav"))
                                {
                                    if let Ok(metadata) = path.metadata() {
                                        let mark = (
                                            metadata.len(),
                                            metadata.modified().unwrap_or(UNIX_EPOCH),
                                        );
                                        if metadata.len() <= 16 * 1024 * 1024
                                            && fingerprints.get(&path) != Some(&mark)
                                        {
                                            fingerprints.insert(path.clone(), mark);
                                            if let Ok(bytes) = std::fs::read(path) {
                                                match decode_wav(&bytes) {
                                                    Ok(audio) => {
                                                        events.push(Event::Sample { name, audio })
                                                    }
                                                    Err(error) => {
                                                        eprintln!("Sample resource: {error}")
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        for (folder, is_stack) in
                            [("registry/stacks", true), ("registry/content", false)]
                        {
                            if let Ok(entries) = std::fs::read_dir(root.join(folder)) {
                                for entry in entries.flatten().take(512) {
                                    let path = entry.path();
                                    let valid = if is_stack {
                                        path.to_string_lossy().ends_with(".scnq.yaml")
                                    } else {
                                        path.extension().is_some_and(|e| e == "json")
                                    };
                                    if !valid {
                                        continue;
                                    }
                                    if let Ok(metadata) = path.metadata() {
                                        let mark = (
                                            metadata.len(),
                                            metadata.modified().unwrap_or(UNIX_EPOCH),
                                        );
                                        if metadata.len() > 1024 * 1024
                                            || fingerprints.get(&path) == Some(&mark)
                                        {
                                            continue;
                                        }
                                        fingerprints.insert(path.clone(), mark);
                                        if let Ok(bytes) = std::fs::read(&path) {
                                            if is_stack {
                                                if let Ok(text) = std::str::from_utf8(&bytes) {
                                                    if let Ok(stack) = crate::scenic::with_plugins(
                                                        controller.current_plugins(),
                                                        || crate::scenic::parse(text),
                                                    ) {
                                                        events.push(Event::Stack {
                                                            name: stack.name.clone(),
                                                            stack,
                                                        });
                                                    }
                                                }
                                            } else if let Ok(document) = serde_json::from_slice::<
                                                crate::storage_formats::Content,
                                            >(
                                                &bytes
                                            ) {
                                                if document.validate().is_err() {
                                                    continue;
                                                }
                                                let entries = document.entries;
                                                events.push(Event::Content {
                                                    name: path
                                                        .file_stem()
                                                        .unwrap_or_default()
                                                        .to_string_lossy()
                                                        .into_owned(),
                                                    entries,
                                                });
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    runtime.block_on(async {
                        for event in events {
                            if stopping.load(Ordering::Relaxed) {
                                break;
                            }
                            if let Err(error) = controller
                                .command(crate::engine::Command::Input { event }, None)
                                .await
                            {
                                eprintln!("Input adapter: {error}")
                            }
                        }
                    });
                    std::thread::sleep(Duration::from_millis(10));
                }
            })
            .map_err(|e| e.to_string())?;
        Ok(Self {
            stop,
            worker: Some(worker),
        })
    }
}
impl Drop for Services {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}
