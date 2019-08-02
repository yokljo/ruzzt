use crate::console::*;

use num::FromPrimitive;
use zzt_file_format::dosstring::DosString;

#[derive(Clone)]
pub struct CaptionState {
	pub text_with_padding: DosString,
	pub time_left: isize,
}

impl CaptionState {
	pub fn new(text: DosString) -> CaptionState {
		let mut text_with_padding = text;
		text_with_padding.data.insert(0, b' ');
		text_with_padding.data.push(b' ');

		CaptionState {
			text_with_padding,
			time_left: 24,
		}
	}

	pub fn draw_caption(&self, console_state: &mut ConsoleState) {
		let fg_num = ((self.time_left - 9) % 7) + 9;
		let fg = ConsoleColour::from_u8(fg_num as u8).unwrap();
		let x = 30 - (self.text_with_padding.len() / 2);
		console_state.draw_text_at(x, 24, &self.text_with_padding.data, ConsoleColour::Black, fg);
	}
}
