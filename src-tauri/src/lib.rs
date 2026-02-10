use std::sync::{Arc, atomic::{AtomicBool, Ordering}};

use cpal::{FromSample, Sample, traits::{DeviceTrait, HostTrait, StreamTrait}};
use dasp::{Signal, signal};
use ringbuf::traits::{Consumer, Observer, Producer, Split};
use tauri::Emitter;

mod filter;
mod ctc_engine;
use ctc_engine::CtcEngine;

struct AppState {
    abort_signal: Arc<AtomicBool>,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct AudioDeviceDescription {
    id: String,
    name: String,
    driver: Option<String>,
    direction: String,
    is_default: bool,
}

#[derive(serde::Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct Payload {
    is_finished: bool,
}

#[derive(serde::Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct PositionCoords {
    left_speaker: [f32; 2],
    right_speaker: [f32; 2],
    left_ear: [f32; 2],
    right_ear: [f32; 2],
}

#[derive(Clone)]
struct ThruOpt<'a> {
    input: &'a cpal::Device,
    output: &'a cpal::Device,
    config: &'a cpal::StreamConfig,
    latency: usize,
    position: PositionCoords,
    master_gain: f32,
    attenuation: f32,
    lowpass_cutoff_min: f32,
    highpass_cutoff: f32,
    lowshelf_cutoff: f32,
    lowshelf_gain: f32,
    wet_dry: f32,
    temperature: f32,
}

trait Coords {
    fn distance(&self, other: Self) -> f32;
}

impl Coords for [f32; 2] {
    fn distance(&self, other: Self) -> f32 {
        let dx = self[0] - other[0];
        let dy = self[1] - other[1];
        dx.hypot(dy)
    }
}

#[tauri::command]
fn get_audio_devices() -> Vec<AudioDeviceDescription> {
    let host = cpal::default_host();
    let devices = host.devices().expect("failed to find devices");
    let default_input_id = host.default_input_device().unwrap().id().unwrap();
    let default_output_id = host.default_output_device().unwrap().id().unwrap();
    devices.map(|d| {
        let id = d.id().expect("failed to get a device id");
        let dd = d.description().expect("failed to get a device description");
        AudioDeviceDescription {
            id: id.1.to_owned(),
            name: dd.name().to_owned(),
            driver: dd.driver().map(|s| s.to_owned()),
            direction: dd.direction().to_string().to_lowercase(),
            is_default: (id == default_input_id) || (id == default_output_id),
        }
    }).collect()
}

#[tauri::command]
fn set_audio_devices(
    window: tauri::Window,
    state: tauri::State<'_, AppState>,
    input_id: &str,
    output_id: &str,
    latency: usize,
    position: PositionCoords,
    master_gain: f32,
    attenuation: f32,
    lowpass_cutoff_min: f32,
    highpass_cutoff: f32,
    lowshelf_cutoff: f32,
    lowshelf_gain: f32,
    wet_dry: f32,
    temperature: f32,
) -> Result<(), ()> {
    let host = cpal::default_host();
    let input_device_id = &cpal::DeviceId(host.id(), input_id.to_owned());
    let output_device_id = &cpal::DeviceId(host.id(), output_id.to_owned());
    let input_device = host.device_by_id(input_device_id).expect("Failed to find an output device");
    let output_device = host.device_by_id(output_device_id).expect("Failed to find an output device");
    let config = input_device.default_input_config().unwrap();

    state.abort_signal.store(false, Ordering::Relaxed);
    let should_abort = Arc::clone(&state.abort_signal);

    let _handler = std::thread::spawn(move || {
        let thru_opt = ThruOpt {
            input: &input_device,
            output: &output_device,
            config: &config.to_owned().into(),
            latency,
            position,
            master_gain,
            attenuation,
            lowpass_cutoff_min,
            highpass_cutoff,
            lowshelf_cutoff,
            lowshelf_gain,
            wet_dry,
            temperature,
        };
        match config.sample_format() {
            cpal::SampleFormat::F32 => start_thru::<f32>(thru_opt, should_abort).unwrap(),
            cpal::SampleFormat::I16 => start_thru::<i16>(thru_opt, should_abort).unwrap(),
            cpal::SampleFormat::U16 => start_thru::<u16>(thru_opt, should_abort).unwrap(),
            _ => panic!("sample format is invalid")
        }
        window.emit("finished", Payload { is_finished: true }).unwrap();
    });

    Ok(())
}

#[tauri::command]
fn abort_audio_routing(state: tauri::State<'_, AppState>) -> Result<(), ()> {
    state.abort_signal.store(true, Ordering::Relaxed);
    Ok(())
}

fn start_thru<T>(opt: ThruOpt<'_>, abort_signal: Arc<AtomicBool>) -> Result<(), ()>
where
    T: cpal::SizedSample + FromSample<f32> + Send + 'static,
    f32: cpal::FromSample<T>,
{
    let sample_rate = opt.config.sample_rate as f32;
    let channels = opt.config.channels as usize;
    
    let latency_frames = opt.latency * (sample_rate as usize) / 1000;
    let latency_samples = latency_frames * channels;

    let rb = ringbuf::HeapRb::<f32>::from(vec![0.0.to_sample::<f32>(); latency_samples]);
    let (mut prod, mut cons) = rb.split();

    let abort_signal_input = Arc::clone(&abort_signal);
    let input_fn = move |data: &[T], _: &cpal::InputCallbackInfo| {
        for &sample in data {
            if prod.try_push(sample.to_sample::<f32>()).is_err() {
                eprintln!("Output stream fell behind; increase latency");
                abort_signal_input.store(true, Ordering::Relaxed);
                break;
            }
        }
    };

    let distances = calc_distance(&opt.position);
    let min_distance = distances.into_iter().reduce(f32::min).unwrap();
    let amp_factors = distances.map(|d| (min_distance / d).powf(1.2) as f64);
    let [main_delays, ct_delays] = calc_delay_frames(
        sample_rate as f32,
        distances,
        calc_speed_of_sound(opt.temperature)
    );
    println!("Delay L/R are {}/{} frames.", ct_delays[0], ct_delays[1]);

    let listenr_pos: [f32; 2] = opt.position.left_ear.iter().zip(opt.position.right_ear).map(|(a, b)| a + b).collect::<Vec<f32>>().try_into().unwrap();
    let shadow_cutoff_l = calc_shadow_cutoff(listenr_pos, opt.position.left_speaker, opt.lowpass_cutoff_min);
    let shadow_cutoff_r = calc_shadow_cutoff(listenr_pos, opt.position.right_speaker, opt.lowpass_cutoff_min);

    let mut engine = CtcEngine::new(
        sample_rate,
        ct_delays,
        main_delays,
        [shadow_cutoff_l, shadow_cutoff_r],
        opt.highpass_cutoff,
        opt.lowshelf_cutoff,
        opt.lowshelf_gain,
    );

    let mut ctc_sig = signal::from_iter(std::iter::from_fn(move || {
        if cons.occupied_len() < 2 { return None; }
        let l = cons.try_pop()? * opt.master_gain;
        let r  = cons.try_pop()? * opt.master_gain;
        Some([l, r])
    })).map(move |[l, r]| {
        let [out_l, out_r] = engine.process([l, r], opt.attenuation as f64, &amp_factors);
        let w = &opt.wet_dry;
        let d = 1.0 - &opt.wet_dry;
        [ out_l * w + l * d, out_r * w + r * d ]
    });

    let output_fn = move |data: &mut [T], _: &cpal::OutputCallbackInfo| {
        for sample in data.chunks_exact_mut(2) {
            let sig = ctc_sig.next();
            if let Some(l) = sample.get_mut(0) {
                *l = sig[0].to_sample();
            }
            if let Some(r) = sample.get_mut(1) {
                *r = sig[1].to_sample();
            }
        }
    };

    let err_fn = |e: cpal::StreamError| {
        eprintln!("Stream error occured: {:?}", e);
    };

    let input_stream = opt.input.build_input_stream(&opt.config, input_fn, err_fn, None).expect("Failed to build input stream");
    let output_stream = opt.output.build_output_stream(&opt.config, output_fn, err_fn, None).expect("Failed to build output stream");

    println!("Started streams with {} ms of latency.", &opt.latency);
    input_stream.play().expect("Failed to play input stream");
    output_stream.play().expect("Failed to play output stream.");

    let dur = std::time::Duration::from_millis(opt.latency as u64);
    while !abort_signal.load(Ordering::Relaxed) {
        std::thread::sleep(dur);
    }

    drop(input_stream);
    drop(output_stream);
    
    println!("Closed safely!");
    Ok(())
}

fn calc_distance(pos: &PositionCoords) -> [f32; 4] {
    [
        pos.left_speaker.distance(pos.left_ear),
        pos.left_speaker.distance(pos.right_ear),
        pos.right_speaker.distance(pos.left_ear),
        pos.right_speaker.distance(pos.right_ear),
    ]
}

fn calc_delay_frames(sample_rate: f32, distances: [f32; 4], speed_of_sound: f64) -> [[f64; 2]; 2] {
    let k = sample_rate as f64 / speed_of_sound;
    let [ls2le, ls2re, rs2le, rs2re] = distances.map(|d| d as f64 * k);
    let main_delays = if ls2le > rs2re { [ 0.0, ls2le - rs2re ] } else { [ rs2re - ls2le, 0.0 ] };
    [
        main_delays,
        [ 1.0f64.max((rs2le - ls2le).abs()), 1.0f64.max((ls2re - rs2re).abs()) ]
    ]
}

fn calc_speed_of_sound(t_c: f32) -> f64 {
    let t_k = 273.15 + t_c;
    (1.403 * 8.314462 * t_k as f64 / 28.966e-3).sqrt()
}

fn calc_shadow_cutoff(coord1: [f32; 2], coord2: [f32; 2], cutoff_min: f32) -> f32 {
    let cutoff_max = 5000.0;
    let diff: Vec<f32> = coord1.iter().zip(coord2).map(|(a, b)| a - b).collect();
    let theta = diff[1].atan2(diff[0]).abs();
    cutoff_min + (cutoff_max - cutoff_min) * theta.cos().powi(2)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let arc= Arc::new(AtomicBool::new(false));
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(AppState {
            abort_signal: Arc::clone(&arc),
        })
        .invoke_handler(tauri::generate_handler![
            get_audio_devices,
            set_audio_devices,
            abort_audio_routing,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}