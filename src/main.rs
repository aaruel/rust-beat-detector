#![feature(associated_consts)]
extern crate portaudio;
extern crate ansi_escapes;

use portaudio as pa;

const SAMPLE_RATE: f64 = 44_100.0;
const FRAMES: u32 = 128;
const CHANNELS: i32 = 2;
const INTERLEAVED: bool = true;

// Mono channel (single energy)
#[derive(Clone)]
struct BeatDetector {
    instant_energy_buffer: Vec<f32>,
    instant_energy_position: usize,
    instant_energy_size: usize,
}

impl BeatDetector {
    // use PA buffer as beat buffer
    const INSTANT_BUFFER_SIZE: usize = FRAMES as usize;

    fn new(sample_buffer_size: usize) -> BeatDetector {
        BeatDetector {
            instant_energy_buffer: vec![0.; sample_buffer_size],
            instant_energy_position: 0,
            instant_energy_size: sample_buffer_size,
        }
    }

    fn sum_instant_energy(&self) -> f32 {
        let energy = |sum, &val| sum + (val * val);
        self.instant_energy_buffer.iter().fold(0., energy)
    }

    fn insert_new_sample(&mut self, sample: f32) {
        // Since buffer size == frames, sum evaluates after each complete chunk
        self.instant_energy_buffer[self.instant_energy_position] = sample;
        self.instant_energy_position = (self.instant_energy_position + 1) % BeatDetector::INSTANT_BUFFER_SIZE;
    }
}

struct BeatDetectorSummer {
    channels: Vec<BeatDetector>,
    energies_buffer: Vec<f32>,
    energies_buffer_position: usize,
    energies_buffer_size: usize,
}

impl BeatDetectorSummer {
    const SENSITIVITY: f32 = 1.3;
    const THRESHOLD: f32 = 1.;

    fn new(num_channels: usize, sample_buffer_size: usize, sample_rate: f32) -> BeatDetectorSummer {
        let ebs = (sample_rate as f32 / sample_buffer_size as f32) as usize;
        BeatDetectorSummer {
            channels: vec![BeatDetector::new(sample_buffer_size); num_channels],
            energies_buffer: vec![0.; ebs],
            energies_buffer_position: 0,
            energies_buffer_size: ebs,
        }
    }

    fn calculate_local_energy(&mut self) -> f32 {
        let float_sbs: f32 = self.energies_buffer_size as f32;
        let ratio: f32 = 1.0 / float_sbs;
        let sum = |sum, &val| sum + (val * val);
        self.energies_buffer.iter().fold(0., sum) * ratio
    }

    fn calculate_instant_energy(&self) -> f32 {
        let mut energy: f32 = 0.;
        for channel in &self.channels {
            energy += channel.sum_instant_energy();
        }
        energy
    }

    fn detect(&mut self) {
        // calculate instant energy
        // calculate local energies
        // insert instant energy into local energy
        // compare instant energy to (constant * local energy)
        let instant_energy = self.calculate_instant_energy();
        let local_energy = self.calculate_local_energy();
        self.energies_buffer[self.energies_buffer_position] = instant_energy;
        self.energies_buffer_position = (self.energies_buffer_position + 1) % self.energies_buffer_size;
        let local_energy_mod = local_energy * BeatDetectorSummer::SENSITIVITY;
        if instant_energy > local_energy_mod && instant_energy > BeatDetectorSummer::THRESHOLD {
            println!("Beat! instant_energy: {} local_energy: {}", instant_energy, local_energy_mod);
        }
    }
}

fn main() {
    match run() {
        Ok(_) => {},
        e => {
            println!("Example failed with the following: {:?}", e);
        }
    }
}

fn run() -> Result<(), pa::Error> {
    let pa = try!(pa::PortAudio::new());

    let default_host = try!(pa.default_host_api());

    let def_input = try!(pa.default_input_device());
    let input_info = try!(pa.device_info(def_input));
    
    // Construct the input stream parameters.
    let latency = input_info.default_low_input_latency;
    let input_params = pa::StreamParameters::<f32>::new(def_input, CHANNELS, INTERLEAVED, latency);

    let def_output = try!(pa.default_output_device());
    let output_info = try!(pa.device_info(def_output));

    // Construct the output stream parameters.
    let latency = output_info.default_low_output_latency;
    let output_params = pa::StreamParameters::new(def_output, CHANNELS, INTERLEAVED, latency);

    // Check that the stream format is supported.
    try!(pa.is_duplex_format_supported(input_params, output_params, SAMPLE_RATE));

    // Construct the settings with which we'll open our duplex stream.
    let settings = pa::DuplexStreamSettings::new(input_params, output_params, SAMPLE_RATE, FRAMES);

    let mut bd_summer = BeatDetectorSummer::new(2, FRAMES as usize, SAMPLE_RATE as f32);

    // A callback to pass to the non-blocking stream.
    let callback = move |pa::DuplexStreamCallbackArgs { in_buffer, out_buffer, frames, .. }| {
        assert!(frames == FRAMES as usize);
        
        let mut i = 0;
        while i < frames * 2 {
            if i & 1 == 0 {
                bd_summer.channels[0].insert_new_sample(in_buffer[i]);
            }
            else {
                bd_summer.channels[1].insert_new_sample(in_buffer[i]);
            }
            out_buffer[i] = in_buffer[i];
            i += 1;
        }
        
        bd_summer.detect();

        pa::Continue
    };

    // Construct a stream with input and output sample types of f32.
    let mut stream = try!(pa.open_non_blocking_stream(settings, callback));

    println!("PortAudio:");
    println!("version: {}", pa.version());
    println!("version text: {:?}", pa.version_text());
    println!("host count: {}", try!(pa.host_api_count()));
    println!("default host: {:#?}", pa.host_api_info(default_host));
    println!("Default input device info: {:#?}", &input_info);
    println!("Default output device info: {:#?}", &output_info);
    println!("");

    try!(stream.start());

    pa.sleep(1000000000);

    try!(stream.stop());

    Ok(())
}