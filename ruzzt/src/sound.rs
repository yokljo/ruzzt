use ruzzt_engine::sounds::*;

use sdl2::audio::{AudioCallback, AudioSpec};
use std::collections::VecDeque;

fn generate_sound_code_frequencies() -> Vec<u16> {
	let mut result = vec![0; 256];
	let c_freq: f64 = 64.;
	for octave in 1 ..= 15 {
		let mut note_freq = c_freq * (2f64.powi(octave as i32 - 1));
		for note in 0..12 {
			result[(note + octave * 16) as usize] = note_freq.floor() as u16;
			note_freq *= 2f64.powf(1. / 12.);
		}
	}
	result
}

pub struct SoundPlayer {
	spec: AudioSpec,
	current_magnitude: f32,
	volume: f32,
	lowpass_level: f32,
	whole_note_samples: usize,
	//last_wave_up: Option<bool>,
	sound_code_frequencies: Vec<u16>,
	sound_entry_queue: VecDeque<SoundEntry>,
	current_frequency: u16,
	current_sound_remaining_samples: usize,
	rendered_samples_to_play: VecDeque<bool>,
	current_priority: SoundPriority,
}

impl SoundPlayer {
	pub fn new(spec: AudioSpec) -> SoundPlayer {
		let whole_note_samples = (spec.freq as f32 * 1.8) as usize;

		SoundPlayer {
			spec,
			current_magnitude: 0.,
			volume: 0.25,
			lowpass_level: 3.,
			whole_note_samples,
			sound_code_frequencies: generate_sound_code_frequencies(),
			sound_entry_queue: VecDeque::new(),
			current_frequency: 0,
			current_sound_remaining_samples: 0,
			rendered_samples_to_play: VecDeque::new(),
			current_priority: SoundPriority::Level(0),
		}
	}

	fn is_sound_playing(&self) -> bool {
		!self.sound_entry_queue.is_empty() || self.current_frequency != 0 || !self.rendered_samples_to_play.is_empty()
	}

	pub fn clear_sound_queue(&mut self) {
		self.sound_entry_queue.clear();
		self.rendered_samples_to_play.clear();
		self.current_frequency = 0;
		self.current_sound_remaining_samples = 0;
	}

	pub fn play_sounds(&mut self, sound_entries: Vec<SoundEntry>, priority: SoundPriority) {
		enum PlayAction {
			None,
			Append,
			Replace,
		};

		let play_action = if self.is_sound_playing() {
			if priority.is_higher_priority_than(&self.current_priority) {
				if priority == SoundPriority::Music {
					PlayAction::Append
				} else {
					PlayAction::Replace
				}
			} else {
				PlayAction::None
			}
		} else {
			PlayAction::Replace
		};

		match play_action {
			PlayAction::None => {}
			PlayAction::Append => {
				self.sound_entry_queue.extend(sound_entries);
			}
			PlayAction::Replace => {
				self.current_priority = priority;
				self.sound_entry_queue.clear();
				self.sound_entry_queue.extend(sound_entries);
			}
		}
	}

	fn play_next_sound(&mut self) {
		if let Some(next_sound) = self.sound_entry_queue.pop_front() {
			if next_sound.sound_code >= 240 {
				let effect_index = next_sound.sound_code - 240;
				self.rendered_samples_to_play.clear();
				for freq in &SOUND_EFFECT_WAVES[effect_index as usize] {
					let half_sample_length = self.spec.freq / *freq as i32 / 2;
					for _ in 0..half_sample_length {
						self.rendered_samples_to_play.push_back(true);
					}
					for _ in 0..half_sample_length {
						self.rendered_samples_to_play.push_back(false);
					}
				}
			} else {
				self.current_frequency = self.sound_code_frequencies[next_sound.sound_code as usize];
			}

			let length_of_32nd_note = self.whole_note_samples / 32;
			self.current_sound_remaining_samples = length_of_32nd_note * next_sound.length_multiplier as usize;
		} else {
			self.current_frequency = 0;
			self.current_sound_remaining_samples = 0;
		}
	}
}

impl AudioCallback for SoundPlayer {
	type Channel = f32;

	fn callback(&mut self, out: &mut [f32]) {
		for sample in out.iter_mut() {
			let dest_mag;
			if let Some(is_up) = self.rendered_samples_to_play.pop_front() {
				dest_mag = if is_up {
					self.volume
				} else {
					-self.volume
				};
			} else if self.current_frequency != 0 {
				let period = self.spec.freq as usize / self.current_frequency as usize;
				let is_up = (self.current_sound_remaining_samples % period) > (period / 2);

				dest_mag = if is_up {
					self.volume
				} else {
					-self.volume
				};
			} else {
				dest_mag = 0.0;
			}

			self.current_magnitude -= (self.current_magnitude - dest_mag) / self.lowpass_level;
			*sample = self.current_magnitude;

			if self.current_sound_remaining_samples == 0 {
				self.play_next_sound();
			} else {
				self.current_sound_remaining_samples -= 1;
			}
		}

		// Generate a square wave
		/*for x in out.iter_mut() {
			*x = if self.phase <= 0.5 {
				self.volume
			} else {
				-self.volume
			};
			self.phase = (self.phase + self.phase_inc) % 1.0;
		}*/

		/*for sample in out.iter_mut() {
			let wave_up;
			if self.note_period_samples > 0 {
				if self.current_sample % self.note_period_samples < (self.note_period_samples as f32 * 0.5) as usize {
					wave_up = Some(true);
				} else {
					wave_up = Some(false);
				}
			} else {
				wave_up = None;
			}

			if wave_up != self.last_wave_up {

			}

			self.current_magnitude -= (self.current_magnitude - dest_mag) / self.lowpass_level;
			*sample = self.current_magnitude;

			self.current_sample += 1;
			if self.current_sample >= self.note_samples {
				self.next_note();
			}
		}*/
	}
}
