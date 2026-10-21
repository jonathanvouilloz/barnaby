//! Local model catalog (spec 6): downloaded on demand, or placed by hand in
//! `models/` and recognised by file name.

use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use anyhow::{anyhow, ensure, Context, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelKind {
    Whisper,
    DiarizationSegmentation,
    DiarizationEmbedding,
    Vad,
}

impl ModelKind {
    /// Matches `ModelKind` in `ui/src/lib/api.ts`.
    pub fn as_str(self) -> &'static str {
        match self {
            ModelKind::Whisper => "whisper",
            ModelKind::DiarizationSegmentation => "diarization_segmentation",
            ModelKind::DiarizationEmbedding => "diarization_embedding",
            ModelKind::Vad => "vad",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelEntry {
    pub id: &'static str,
    pub kind: ModelKind,
    pub name: &'static str,
    /// Download size in MB (10^6 bytes).
    pub size_mb: u32,
    pub url: &'static str,
    /// Path inside `models/`. For a `.tar.bz2` download it is also the archive member
    /// kept, so an archive unpacked by hand into `models/` lands on the same path.
    pub file_name: &'static str,
}

use ModelKind::*;

static CATALOG: &[ModelEntry] = &[
    ModelEntry { id: "whisper-tiny", kind: Whisper, name: "Whisper tiny", size_mb: 78, url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.bin", file_name: "ggml-tiny.bin" },
    ModelEntry { id: "whisper-base", kind: Whisper, name: "Whisper base", size_mb: 148, url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.bin", file_name: "ggml-base.bin" },
    ModelEntry { id: "whisper-small", kind: Whisper, name: "Whisper small", size_mb: 488, url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-small.bin", file_name: "ggml-small.bin" },
    ModelEntry {
        id: "whisper-large-v3-turbo",
        kind: Whisper,
        name: "Whisper large-v3 turbo",
        size_mb: 1625,
        url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-large-v3-turbo.bin",
        file_name: "ggml-large-v3-turbo.bin",
    },
    ModelEntry {
        id: "pyannote-segmentation-3-0",
        kind: DiarizationSegmentation,
        name: "Pyannote segmentation 3.0",
        size_mb: 7,
        url: "https://github.com/k2-fsa/sherpa-onnx/releases/download/speaker-segmentation-models/sherpa-onnx-pyannote-segmentation-3-0.tar.bz2",
        file_name: "sherpa-onnx-pyannote-segmentation-3-0/model.onnx",
    },
    // WeSpeaker ResNet34 trained on VoxCeleb (speakers of 145 nationalities, many languages)
    // with large-margin fine-tuning; speaker embeddings depend little on the spoken language.
    ModelEntry {
        id: "wespeaker-voxceleb-resnet34-lm",
        kind: DiarizationEmbedding,
        name: "WeSpeaker ResNet34 (VoxCeleb)",
        size_mb: 27,
        url: "https://github.com/k2-fsa/sherpa-onnx/releases/download/speaker-recongition-models/wespeaker_en_voxceleb_resnet34_LM.onnx",
        file_name: "wespeaker_en_voxceleb_resnet34_LM.onnx",
    },
    ModelEntry { id: "silero-vad", kind: Vad, name: "Silero VAD", size_mb: 1, url: "https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/silero_vad.onnx", file_name: "silero_vad.onnx" },
];

pub fn catalog() -> &'static [ModelEntry] {
    CATALOG
}

fn find(id: &str) -> Option<&'static ModelEntry> {
    CATALOG.iter().find(|m| m.id == id)
}

pub fn path_for(models_dir: &Path, id: &str) -> Option<PathBuf> {
    find(id).map(|m| models_dir.join(m.file_name)).filter(|p| p.is_file())
}

pub fn installed(models_dir: &Path, id: &str) -> bool {
    path_for(models_dir, id).is_some()
}

/// Blocking download of model `id` into `models_dir`; `progress` gets a fraction in [0, 1].
/// The file only appears under its final name once complete and size-checked.
pub fn download(models_dir: &Path, id: &str, progress: impl Fn(f32)) -> Result<PathBuf> {
    let entry = find(id).ok_or_else(|| anyhow!("unknown model {id}"))?;
    fetch(models_dir, entry, &progress)
}

fn fetch(models_dir: &Path, entry: &ModelEntry, progress: &dyn Fn(f32)) -> Result<PathBuf> {
    let dest = models_dir.join(entry.file_name);
    let part = PathBuf::from(format!("{}.part", dest.display()));
    let archive = entry.url.ends_with(".tar.bz2");
    let download = if archive { models_dir.join(format!("{}.tar.bz2.part", entry.id)) } else { part.clone() };
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent)?;
    }
    let result = (|| {
        stream_to(entry, &download, progress)?;
        if archive {
            extract(&download, entry.file_name, &part)?;
            fs::remove_file(&download)?;
        }
        fs::rename(&part, &dest)?;
        progress(1.0);
        Ok(dest.clone())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&download);
        let _ = fs::remove_file(&part);
    }
    result
}

fn stream_to(entry: &ModelEntry, path: &Path, progress: &dyn Fn(f32)) -> Result<()> {
    let mut resp = ureq::get(entry.url).call().with_context(|| format!("download {}", entry.id))?;
    let expected = u64::from(entry.size_mb) * 1_000_000;
    let length = resp.body().content_length();
    let total = length.unwrap_or(expected).max(1);
    let mut reader = resp.body_mut().as_reader();
    let mut file = File::create(path)?;
    let mut buf = vec![0u8; 1 << 16];
    let mut done: u64 = 0;
    loop {
        let n = reader.read(&mut buf)?;
        if n == 0 {
            break;
        }
        file.write_all(&buf[..n])?;
        done += n as u64;
        progress((done as f32 / total as f32).min(1.0));
    }
    file.sync_all()?;
    ensure!(length.is_none_or(|l| l == done), "download {} truncated", entry.id);
    // Catches an error page or a different file served under the same URL, while
    // tolerating a minor upstream re-upload.
    ensure!(done.abs_diff(expected) <= (expected / 10).max(1_000_000), "download {}: unexpected size {done}", entry.id);
    Ok(())
}

fn extract(archive: &Path, member: &str, out: &Path) -> Result<()> {
    let mut tar = tar::Archive::new(bzip2::read::BzDecoder::new(File::open(archive)?));
    for item in tar.entries()? {
        let mut item = item?;
        if item.path()? == Path::new(member) {
            std::io::copy(&mut item, &mut File::create(out)?)?;
            return Ok(());
        }
    }
    Err(anyhow!("{member} missing from archive"))
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use std::io::BufRead;
    use std::net::TcpListener;
    use std::sync::Mutex;

    /// Shared by the `#[ignore]` model tests; downloads what they need once.
    pub fn models_dir() -> PathBuf {
        std::env::var_os("BARNABY_MODELS_DIR").map(PathBuf::from).unwrap_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/test-models"))
    }

    pub fn ensure(id: &str) -> PathBuf {
        let dir = models_dir();
        path_for(&dir, id).unwrap_or_else(|| download(&dir, id, |_| {}).unwrap())
    }

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("barnaby-models-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// Serves `body` once over HTTP on loopback; returns the URL.
    fn serve(body: Vec<u8>, path: &str) -> &'static str {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}/{path}", listener.local_addr().unwrap());
        std::thread::spawn(move || {
            let (mut conn, _) = listener.accept().unwrap();
            let mut reader = std::io::BufReader::new(conn.try_clone().unwrap());
            let mut line = String::new();
            while reader.read_line(&mut line).unwrap() > 2 {
                line.clear();
            }
            write!(conn, "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len()).unwrap();
            conn.write_all(&body).unwrap();
        });
        Box::leak(url.into_boxed_str())
    }

    fn fake(url: &'static str, size_mb: u32, file_name: &'static str) -> ModelEntry {
        ModelEntry { id: "fake", kind: Whisper, name: "Fake", size_mb, url, file_name }
    }

    #[test]
    fn catalog_ids_are_unique_and_detected_by_file_name() {
        let dir = temp_dir("detect");
        let mut ids: Vec<_> = catalog().iter().map(|m| m.id).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), catalog().len());
        assert!(catalog().iter().any(|m| m.id == "whisper-large-v3-turbo" && m.kind.as_str() == "whisper"));

        assert!(!installed(&dir, "whisper-base"));
        fs::write(dir.join("ggml-base.bin"), b"x").unwrap();
        assert!(installed(&dir, "whisper-base"));
        assert_eq!(path_for(&dir, "whisper-base"), Some(dir.join("ggml-base.bin")));
        assert!(!installed(&dir, "nope"));

        fs::create_dir_all(dir.join("sherpa-onnx-pyannote-segmentation-3-0")).unwrap();
        fs::write(dir.join("sherpa-onnx-pyannote-segmentation-3-0/model.onnx"), b"x").unwrap();
        assert!(installed(&dir, "pyannote-segmentation-3-0"));
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn download_reports_progress_and_renames() {
        let dir = temp_dir("plain");
        let body = vec![7u8; 300_000];
        let seen = Mutex::new(Vec::new());
        let entry = fake(serve(body.clone(), "m.bin"), 0, "m.bin");
        let path = fetch(&dir, &entry, &|p| seen.lock().unwrap().push(p)).unwrap();
        assert_eq!(fs::read(&path).unwrap(), body);
        assert!(!dir.join("m.bin.part").exists());
        let seen = seen.into_inner().unwrap();
        assert!(seen.windows(2).all(|w| w[0] <= w[1]) && seen.last() == Some(&1.0) && seen.len() > 2);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn download_extracts_member_from_tar_bz2() {
        let dir = temp_dir("archive");
        let mut tar = tar::Builder::new(bzip2::write::BzEncoder::new(Vec::new(), bzip2::Compression::fast()));
        for (name, data) in [("seg/README", &b"readme"[..]), ("seg/model.onnx", &b"onnx-bytes"[..])] {
            let mut header = tar::Header::new_gnu();
            header.set_size(data.len() as u64);
            header.set_mode(0o644);
            tar.append_data(&mut header, name, data).unwrap();
        }
        let archive = tar.into_inner().unwrap().finish().unwrap();
        let entry = fake(serve(archive, "seg.tar.bz2"), 0, "seg/model.onnx");
        let path = fetch(&dir, &entry, &|_| {}).unwrap();
        assert_eq!(path, dir.join("seg/model.onnx"));
        assert_eq!(fs::read(path).unwrap(), b"onnx-bytes");
        assert_eq!(fs::read_dir(&dir).unwrap().count(), 1, "only seg/ is left");
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn wrong_size_leaves_nothing_behind() {
        let dir = temp_dir("size");
        let entry = fake(serve(b"<html>not found</html>".to_vec(), "big.bin"), 50, "big.bin");
        assert!(fetch(&dir, &entry, &|_| {}).is_err());
        assert_eq!(fs::read_dir(&dir).unwrap().count(), 0);
        fs::remove_dir_all(dir).unwrap();
    }
}
