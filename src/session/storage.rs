use super::{Session, MAX_BYTES};
use crate::model::Result;
use serde::Serialize;
use std::{
    fs::{self, File},
    io::{BufWriter, Read, Write},
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicU64, Ordering},
        Mutex,
    },
};
pub fn load(path: &Path) -> Result<Session> {
    let file = File::open(path).map_err(|e| e.to_string())?;
    if file.metadata().map_err(|e| e.to_string())?.len() > MAX_BYTES {
        return Err("Session exceeds 64 MiB".into());
    }
    let mut bytes = Vec::new();
    file.take(MAX_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() as u64 > MAX_BYTES {
        return Err("Session exceeds 64 MiB".into());
    }
    super::decode(&bytes)
}

#[derive(Clone, Default, Serialize)]
pub struct Status {
    pub enabled: bool,
    pub saved_revision: Option<u64>,
    pub saved_frame: Option<u64>,
    pub error: Option<String>,
}
struct State {
    fingerprint: Option<(u64, std::time::SystemTime)>,
    status: Status,
}
pub struct Store {
    path: PathBuf,
    state: Mutex<State>,
}
impl Store {
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            state: Mutex::new(State {
                fingerprint: None,
                status: Status {
                    enabled: true,
                    ..Status::default()
                },
            }),
        }
    }
    pub fn status(&self) -> Status {
        self.state
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .status
            .clone()
    }
    pub fn save(&self, session: &Session) -> Result<()> {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        if state
            .status
            .saved_revision
            .zip(state.status.saved_frame)
            .is_some_and(|saved| saved > (session.snapshot.revision, session.snapshot.frame))
        {
            return Ok(());
        }
        let result = session
            .validate()
            .and_then(|_| self.write(session, state.fingerprint));
        state.status.error = result.as_ref().err().cloned();
        if result.is_ok() {
            state.fingerprint = fingerprint(&self.path);
            state.status.saved_revision = Some(session.snapshot.revision);
            state.status.saved_frame = Some(session.snapshot.frame);
        }
        result
    }
    fn write(
        &self,
        session: &Session,
        verified: Option<(u64, std::time::SystemTime)>,
    ) -> Result<()> {
        let parent = self.path.parent().ok_or("Session path has no parent")?;
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let temp = parent.join(format!(
            ".session-{}-{}.tmp",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        struct Limited<W> {
            writer: W,
            bytes: u64,
        }
        impl<W: Write> Write for Limited<W> {
            fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
                if self.bytes + buf.len() as u64 > MAX_BYTES {
                    return Err(std::io::Error::other("Session exceeds 64 MiB"));
                }
                let n = self.writer.write(buf)?;
                self.bytes += n as u64;
                Ok(n)
            }
            fn flush(&mut self) -> std::io::Result<()> {
                self.writer.flush()
            }
        }
        let result = (|| {
            let file = File::create(&temp).map_err(|e| e.to_string())?;
            let mut writer = BufWriter::new(Limited {
                writer: file,
                bytes: 0,
            });
            serde_json::to_writer(&mut writer, session).map_err(|e| e.to_string())?;
            writer.flush().map_err(|e| e.to_string())?;
            writer
                .get_ref()
                .writer
                .sync_all()
                .map_err(|e| e.to_string())?;
            drop(writer);
            if verified.is_none() || verified != fingerprint(&self.path) {
                crate::migrations::backup_existing(&self.path, crate::migrations::Kind::Session)?;
            }
            fs::rename(&temp, &self.path).map_err(|e| e.to_string())?;
            #[cfg(unix)]
            File::open(parent)
                .and_then(|directory| directory.sync_all())
                .map_err(|e| e.to_string())?;
            Ok(())
        })();
        if result.is_err() {
            let _ = fs::remove_file(&temp);
        }
        result
    }
}

fn fingerprint(path: &Path) -> Option<(u64, std::time::SystemTime)> {
    let metadata = fs::metadata(path).ok()?;
    Some((metadata.len(), metadata.modified().ok()?))
}
