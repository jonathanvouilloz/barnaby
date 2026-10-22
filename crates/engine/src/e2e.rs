//! Full local pipeline with the real providers: whisper.cpp + sherpa-onnx through `barnaby-core`.

use std::sync::{Arc, Mutex};

use barnaby_core::{
    pipeline::Pipeline,
    retention::{meeting_dir, MIC_WAV, MIX_WAV, SYSTEM_WAV},
    store::Store,
    MeetingState, ProcessingMode,
};

use crate::{models::tests::ensure, SherpaDiarizer, WhisperTranscriber};

fn write_wav(path: &std::path::Path, samples: &[f32]) {
    let spec = hound::WavSpec { channels: 1, sample_rate: 16_000, bits_per_sample: 16, sample_format: hound::SampleFormat::Int };
    let mut w = hound::WavWriter::create(path, spec).unwrap();
    samples.iter().for_each(|s| w.write_sample((s * 32767.0) as i16).unwrap());
    w.finalize().unwrap();
}

/// Downloads whisper-tiny, the sherpa models and whisper.cpp's 11 s speech sample.
#[test]
#[ignore]
fn local_pipeline_attributes_mic_and_system_speech() {
    let mut resp = ureq::get("https://raw.githubusercontent.com/ggml-org/whisper.cpp/master/samples/jfk.wav").call().unwrap();
    let wav = resp.body_mut().read_to_vec().unwrap();
    let speech: Vec<f32> = hound::WavReader::new(std::io::Cursor::new(wav)).unwrap().into_samples::<i16>().map(|s| f32::from(s.unwrap()) / 32768.0).collect();
    let silence = vec![0.0; speech.len()];
    // The user speaks first (mic), then a remote participant (system track).
    let mic = [speech.as_slice(), &silence].concat();
    let system = [silence.as_slice(), &speech].concat();

    let data_dir = std::env::temp_dir().join(format!("barnaby-e2e-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&data_dir);
    let store = Store::open_in_memory().unwrap();
    let id = store.create_meeting("e2e", &[], ProcessingMode::Local).unwrap().id;
    let dir = meeting_dir(&data_dir, id);
    std::fs::create_dir_all(&dir).unwrap();
    write_wav(&dir.join(MIC_WAV), &mic);
    write_wav(&dir.join(SYSTEM_WAV), &system);
    store.set_state(id, MeetingState::Queued, None).unwrap();

    let store = Arc::new(Mutex::new(store));
    let pipeline = Pipeline {
        store: store.clone(),
        transcriber: Arc::new(WhisperTranscriber::load(&ensure("whisper-tiny"), false).unwrap()),
        diarizer: Arc::new(SherpaDiarizer::load(&ensure("pyannote-segmentation-3-0"), &ensure("wespeaker-voxceleb-resnet34-lm")).unwrap()),
        structurer: None,
        data_dir: data_dir.clone(),
    };
    pipeline.run(id).unwrap();

    let db = store.lock().unwrap();
    assert_eq!(db.get_meeting(id).unwrap().state, MeetingState::NeedsReview);
    let doc = db.meeting_doc(id).unwrap();
    let half = speech.len() as f64 / 16_000.0;
    let mine: Vec<_> = doc.segments.iter().filter(|s| s.end <= half + 0.5).collect();
    let theirs: Vec<_> = doc.segments.iter().filter(|s| s.start >= half - 0.5).collect();
    assert!(!mine.is_empty() && !theirs.is_empty(), "segments: {:?}", doc.segments);
    assert!(mine.iter().all(|s| s.speaker_id.as_deref() == Some("me")), "{mine:?}");
    assert!(theirs.iter().all(|s| s.speaker_id.as_deref().is_some_and(|id| id != "me")), "{theirs:?}");
    assert!(doc.segments.iter().any(|s| s.text.to_lowercase().contains("country")));
    assert!(dir.join(MIX_WAV).exists());
    // Idempotent: a second run keeps the same segments.
    drop(db);
    pipeline.run(id).unwrap();
    assert_eq!(store.lock().unwrap().meeting_doc(id).unwrap().segments, doc.segments);
    let _ = std::fs::remove_dir_all(&data_dir);
}
