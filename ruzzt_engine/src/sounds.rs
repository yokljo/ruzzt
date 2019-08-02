use lazy_static::lazy_static;

lazy_static! {
	/// When playing a sound effect, it takes one of these arrays depending on the selected sound effect
	/// index, then it toggles the speaker for (1/sample) seconds. Eg. for index 1, the speaker might be
	/// up for 1/1100 seconds, then down for 1/1200 seconds, then up for 1/1300 seconds, and so forth.
	pub static ref SOUND_EFFECT_WAVES: Vec<Vec<u16>> = vec![
		vec![3200],
		vec![1100, 1200, 1300, 1400, 1500, 1600, 1700, 1800, 1900, 2000, 2100, 2200, 2300, 2400],
		vec![4800, 4800, 8000, 1600, 4800, 4800, 8000, 1600, 4800, 4800, 8000, 1600, 4800, 4800],
		vec![0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
		vec![500, 2556, 1929, 3776, 3386, 4517, 1385, 1103, 4895, 3396, 874, 1616, 5124, 606],
		vec![1600, 1514, 1600, 821, 1600, 1715, 1600, 911, 1600, 1968, 1600, 1490, 1600, 1722],
		vec![2200, 1760, 1760, 1320, 2640, 880, 2200, 1760, 1760, 1320, 2640, 880, 2200, 1760],
		vec![688, 676, 664, 652, 640, 628, 616, 604, 592, 580, 568, 556, 544, 532],
		vec![1207, 1224, 1163, 1127, 1159, 1236, 1269, 1314, 1127, 1224, 1320, 1332, 1257, 1327],
		vec![378, 331, 316, 230, 224, 384, 480, 320, 358, 412, 376, 621, 554, 426],
	];
}

/// The priority of a sound that will be added to the sound player. Music is appended to whatever is
/// currently playing. Sounds with higher levels will replace currently playing sounds, and lower
/// levels will be ignored.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SoundPriority {
	Music,
	Level(usize),
}

impl SoundPriority {
	/// Returns true if this priority is higher than the `other`, or is music (and will get appended
	/// to whatever is playing currently).
	pub fn is_higher_priority_than(&self, other: &SoundPriority) -> bool {
		match (self, other) {
			(SoundPriority::Level(self_level), SoundPriority::Level(other_level)) => {
				self_level >= other_level
			}
			(SoundPriority::Music, _) => true,
			_ => false,
		}
	}
}

/// A single note or sound effect that can be stringed together to make game sounds.
#[derive(Debug, Clone, PartialEq)]
pub struct SoundEntry {
	/// The code of the sound to play. 0-239 are notes, and 240-255 are sound effects from the
	/// `SOUND_EFFECT_WAVES` list.
	pub sound_code: u8,
	// 1 means 32nd note, 2 means 16th note...
	pub length_multiplier: u8,
}

/// Get a notes string as written in ZZT OOP, and convert it to a list of `SoundEntry` (which is
/// what the sound player actually accepts).
pub fn process_notes_string(notes_string: &[u8]) -> Vec<SoundEntry> {
	let mut current_note_index = 0;
	let mut octave_offset = 3;
	let mut length_multiplier = 1;
	let mut result = vec![];

	while current_note_index < notes_string.len() {
		match notes_string[current_note_index].to_ascii_lowercase() {
			b't' => {
				length_multiplier = 1;
			}
			b's' => {
				length_multiplier = 2;
			}
			b'i' => {
				length_multiplier = 4;
			}
			b'q' => {
				length_multiplier = 8;
			}
			b'h' => {
				length_multiplier = 16;
			}
			b'w' => {
				length_multiplier = 32;
			}
			b'3' => {
				length_multiplier /= 3;
			}
			b'.' => {
				length_multiplier = length_multiplier + (length_multiplier / 2);
			}
			b'+' => {
				if octave_offset < 6 {
					octave_offset += 1
				}
			}
			b'-' => {
				if octave_offset > 1 {
					octave_offset -= 1
				}
			}
			b'x' => {
				result.push(SoundEntry{
					sound_code: 0,
					length_multiplier,
				});
			}
			note_name @ b'a' ..= b'g' => {
				let scale_indices: [u8; 7] = [9, 11, 0, 2, 4, 5, 7];
				let mut scale_index: u8 = scale_indices[(note_name - b'a') as usize];

				if let Some(sharp_flat) = notes_string.get(current_note_index + 1) {
					match sharp_flat {
						b'#' => {
							scale_index = scale_index.wrapping_add(1);
							current_note_index += 1;
						}
						b'!' => {
							scale_index = scale_index.wrapping_sub(1);
							current_note_index += 1;
						}
						_ => {}
					}
				}

				let sound_code = octave_offset * 16 + scale_index;

				result.push(SoundEntry{
					sound_code,
					length_multiplier,
				});
			}
			// This doesn't include b'3', which is matched above.
			sound_effect_char @ b'0'..= b'9' => {
				let sound_effect_index = sound_effect_char - b'0';
				let sound_code = sound_effect_index + 240;

				result.push(SoundEntry{
					sound_code,
					length_multiplier,
				});
			}
			_ => {}
		}

		current_note_index += 1;
	}

	result
}
