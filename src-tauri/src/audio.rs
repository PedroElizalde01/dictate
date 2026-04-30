use anyhow::{anyhow, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{SampleFormat, SampleRate, Stream, StreamConfig};
use parking_lot::Mutex;
use serde::Serialize;
use std::path::Path;
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};

#[derive(Debug, Serialize, Clone)]
pub struct MicDevice {
    pub name: String,
    pub is_default: bool,
}

pub fn list_input_devices() -> Result<Vec<MicDevice>> {
    let host = cpal::default_host();
    let default_name = host.default_input_device().and_then(|d| d.name().ok());
    let mut out = Vec::new();
    for d in host.input_devices()? {
        let name = d.name().unwrap_or_else(|_| "unknown".into());
        let is_default = Some(&name) == default_name.as_ref();
        out.push(MicDevice { name, is_default });
    }
    Ok(out)
}

fn pick_device(name: &Option<String>) -> Result<cpal::Device> {
    let host = cpal::default_host();
    if let Some(n) = name {
        for d in host.input_devices()? {
            if d.name().unwrap_or_default() == *n {
                return Ok(d);
            }
        }
    }
    host.default_input_device()
        .ok_or_else(|| anyhow!("no default input device"))
}

enum Cmd {
    Start {
        mic: Option<String>,
        reply: mpsc::Sender<Result<(), String>>,
    },
    Stop {
        reply: mpsc::Sender<Result<(Vec<f32>, u32), String>>,
    },
}

#[derive(Clone)]
pub struct AudioController {
    cmd_tx: mpsc::Sender<Cmd>,
}

impl AudioController {
    pub fn spawn(app: AppHandle) -> Self {
        let (tx, rx) = mpsc::channel::<Cmd>();
        thread::spawn(move || run_loop(app, rx));
        Self { cmd_tx: tx }
    }

    pub fn start(&self, mic: Option<String>) -> Result<()> {
        let (rtx, rrx) = mpsc::channel();
        self.cmd_tx
            .send(Cmd::Start { mic, reply: rtx })
            .map_err(|_| anyhow!("audio thread gone"))?;
        rrx.recv()
            .map_err(|_| anyhow!("audio thread dropped reply"))?
            .map_err(|e| anyhow!(e))
    }

    pub fn stop(&self) -> Result<(Vec<f32>, u32)> {
        let (rtx, rrx) = mpsc::channel();
        self.cmd_tx
            .send(Cmd::Stop { reply: rtx })
            .map_err(|_| anyhow!("audio thread gone"))?;
        rrx.recv()
            .map_err(|_| anyhow!("audio thread dropped reply"))?
            .map_err(|e| anyhow!(e))
    }
}

struct Active {
    stream: Stream,
    buffer: Arc<Mutex<Vec<f32>>>,
    src_rate: u32,
}

fn run_loop(app: AppHandle, rx: mpsc::Receiver<Cmd>) {
    let mut active: Option<Active> = None;
    while let Ok(cmd) = rx.recv() {
        match cmd {
            Cmd::Start { mic, reply } => {
                if active.is_some() {
                    let _ = reply.send(Err("already recording".into()));
                    continue;
                }
                match build_stream(&app, mic) {
                    Ok(a) => {
                        active = Some(a);
                        let _ = reply.send(Ok(()));
                    }
                    Err(e) => {
                        let _ = reply.send(Err(format!("{e}")));
                    }
                }
            }
            Cmd::Stop { reply } => match active.take() {
                None => {
                    let _ = reply.send(Err("not recording".into()));
                }
                Some(a) => {
                    let _ = a.stream.pause();
                    thread::sleep(Duration::from_millis(60));
                    drop(a.stream);
                    let data = std::mem::take(&mut *a.buffer.lock());
                    let _ = reply.send(Ok((data, a.src_rate)));
                }
            },
        }
    }
}

fn build_stream(app: &AppHandle, mic: Option<String>) -> Result<Active> {
    let device = pick_device(&mic)?;
    let supported = device.default_input_config()?;
    let src_rate = supported.sample_rate().0;
    let src_channels = supported.channels();
    let format = supported.sample_format();
    let config: StreamConfig = StreamConfig {
        channels: src_channels,
        sample_rate: SampleRate(src_rate),
        buffer_size: cpal::BufferSize::Default,
    };
    let buffer: Arc<Mutex<Vec<f32>>> =
        Arc::new(Mutex::new(Vec::with_capacity(src_rate as usize * 10)));
    let frame_state: Arc<Mutex<(f32, usize, Instant)>> =
        Arc::new(Mutex::new((0.0, 0, Instant::now())));
    let channels = src_channels as usize;
    let err_fn = |e| eprintln!("audio stream error: {e}");

    let stream = match format {
        SampleFormat::F32 => {
            let buf = buffer.clone();
            let fs = frame_state.clone();
            let app2 = app.clone();
            device.build_input_stream(
                &config,
                move |data: &[f32], _| push_samples(&buf, &fs, &app2, data, channels, |s| s),
                err_fn,
                None,
            )?
        }
        SampleFormat::I16 => {
            let buf = buffer.clone();
            let fs = frame_state.clone();
            let app2 = app.clone();
            device.build_input_stream(
                &config,
                move |data: &[i16], _| {
                    push_samples(&buf, &fs, &app2, data, channels, |s| s as f32 / 32768.0)
                },
                err_fn,
                None,
            )?
        }
        SampleFormat::U16 => {
            let buf = buffer.clone();
            let fs = frame_state.clone();
            let app2 = app.clone();
            device.build_input_stream(
                &config,
                move |data: &[u16], _| {
                    push_samples(&buf, &fs, &app2, data, channels, |s| {
                        (s as f32 - 32768.0) / 32768.0
                    })
                },
                err_fn,
                None,
            )?
        }
        f => return Err(anyhow!("unsupported sample format {:?}", f)),
    };
    stream.play()?;
    Ok(Active {
        stream,
        buffer,
        src_rate,
    })
}

fn push_samples<T: Copy>(
    buf: &Arc<Mutex<Vec<f32>>>,
    frame_state: &Arc<Mutex<(f32, usize, Instant)>>,
    app: &AppHandle,
    data: &[T],
    channels: usize,
    conv: impl Fn(T) -> f32,
) {
    let mut b = buf.lock();
    let mut fs = frame_state.lock();
    for chunk in data.chunks(channels) {
        let s: f32 = chunk.iter().map(|&v| conv(v)).sum::<f32>() / channels as f32;
        b.push(s);
        fs.0 += s * s;
        fs.1 += 1;
    }
    if fs.2.elapsed().as_millis() > 50 && fs.1 > 0 {
        let rms = (fs.0 / fs.1 as f32).sqrt();
        let _ = app.emit("audio-level", (rms * 4.0).min(1.0));
        fs.0 = 0.0;
        fs.1 = 0;
        fs.2 = Instant::now();
    }
}

pub fn write_wav_16k(path: &Path, samples: &[f32], src_rate: u32) -> Result<()> {
    let resampled = if src_rate == 16000 {
        samples.to_vec()
    } else {
        resample_to_16k(samples, src_rate)
    };
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: 16000,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut w = hound::WavWriter::create(path, spec)?;
    for &s in &resampled {
        let v = (s.clamp(-1.0, 1.0) * 32767.0) as i16;
        w.write_sample(v)?;
    }
    w.finalize()?;
    Ok(())
}

fn resample_to_16k(input: &[f32], src_rate: u32) -> Vec<f32> {
    let ratio = 16000.0 / src_rate as f32;
    let out_len = (input.len() as f32 * ratio) as usize;
    let mut out = Vec::with_capacity(out_len);
    for i in 0..out_len {
        let src_pos = i as f32 / ratio;
        let idx = src_pos as usize;
        let frac = src_pos - idx as f32;
        let a = input.get(idx).copied().unwrap_or(0.0);
        let b = input.get(idx + 1).copied().unwrap_or(a);
        out.push(a + (b - a) * frac);
    }
    out
}
