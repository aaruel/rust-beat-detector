#![feature(associated_consts)]
extern crate portaudio;
extern crate ansi_escapes;

use portaudio as pa;

const SAMPLE_RATE: f64 = 44_100.0;
const FRAMES: u32 = 128;
const CHANNELS: i32 = 2;
const INTERLEAVED: bool = true;

mod atk;

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

    let mut bd_summer = atk::beatdetector::BeatDetectorSummer::new(2, FRAMES as usize, SAMPLE_RATE as f32);

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