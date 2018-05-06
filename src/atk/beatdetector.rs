// Mono channel (single energy)
#[derive(Clone)]
struct BeatDetectorChannel {
    instant_energy_buffer: Vec<f32>,
    instant_energy_position: usize,
    instant_energy_size: usize,
}

impl BeatDetectorChannel {
    fn new(sample_buffer_size: usize) -> BeatDetectorChannel {
        BeatDetectorChannel {
            instant_energy_buffer: vec![0.; sample_buffer_size],
            instant_energy_position: 0,
            instant_energy_size: sample_buffer_size,
        }
    }

    fn sum_instant_energy(&self) -> f32 {
        let energy = |sum, &val| sum + (val * val);
        self.instant_energy_buffer.iter().fold(0., energy)
    }

    fn insert_sample(&mut self, sample: f32) {
        // Since buffer size == frames, sum evaluates after each complete chunk
        self.instant_energy_buffer[self.instant_energy_position] = sample;
        self.instant_energy_position = (self.instant_energy_position + 1) % self.instant_energy_size;
    }
}

pub struct BeatDetectorSummer {
    channels: Vec<BeatDetectorChannel>,
    energies_buffer: Vec<f32>,
    energies_buffer_position: usize,
    energies_buffer_size: usize,
}

impl BeatDetectorSummer {
    const SENSITIVITY: f32 = 1.3;
    const THRESHOLD: f32 = 1.;

    pub fn new(num_channels: usize, sample_buffer_size: usize, sample_rate: f32) -> BeatDetectorSummer {
        let ebs = (sample_rate as f32 / sample_buffer_size as f32) as usize;
        BeatDetectorSummer {
            channels: vec![BeatDetectorChannel::new(sample_buffer_size); num_channels],
            energies_buffer: vec![0.; ebs],
            energies_buffer_position: 0,
            energies_buffer_size: ebs,
        }
    }

    pub fn insert_sample(&mut self, channel: usize, sample: f32) {
        assert!(
            channel < self.channels.len(), 
            "Channel index {} is out of range", channel
        );
        self.channels[channel].insert_sample(sample);
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

    pub fn detect(&mut self) {
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