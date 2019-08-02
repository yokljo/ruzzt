use crate::event::*;
use crate::board_message::*;
use crate::console::*;
use zzt_file_format::{WorldHeader, BoardMetaData};
use zzt_file_format::dosstring::DosString;

use num::FromPrimitive;

/// When a text input is open in the side bar, this represents the purpose of the input.
#[derive(Clone)]
pub enum TextInputMode {
	SaveFile,
	Debug,
}

impl TextInputMode {
	/// The maximum number of characters allowed in the input mode.
	fn max_char_len(&self) -> usize {
		match self {
			TextInputMode::SaveFile => 8,
			TextInputMode::Debug => 11,
		}
	}

	/// The suffix string to display at the end of the input box.
	fn suffix(&self) -> &'static [u8] {
		match self {
			TextInputMode::SaveFile => b".SAV",
			TextInputMode::Debug => b"",
		}
	}

	/// Whether the input text is forced to be upper-case.
	fn force_upper(&self) -> bool {
		match self {
			TextInputMode::SaveFile => true,
			TextInputMode::Debug => false,
		}
	}
}

/// A TextInputMode and the text that is in the input box, if one is open in the side bar.
#[derive(Clone)]
struct TextInputState {
	mode: TextInputMode,
	text: DosString,
}

/// If there is a yes/no question open in the side bar, this represents the purpose of that yes/no
/// question.
#[derive(Clone)]
pub enum YesNoMode {
	EndGame,
	Quit,
}

/// If a text-based input is open in the side bar, this contains the state of that input.
#[derive(Clone)]
enum InputState {
	Text(TextInputState),
	YesNo(YesNoMode),
}

/// The state of the side bar.
#[derive(Clone)]
pub struct SideBar {
	/// If there is a text or yes/no input open in the side bar, this contains the mode and state
	/// of that input.
	input_state: Option<InputState>,
}

impl SideBar {
	/// Make a new side bar with no input open.
	pub fn new() -> SideBar {
		SideBar {
			input_state: None,
		}
	}

	/// True if an input is open in the side bar, meaning `process_typing` should be called with key
	/// press events.
	pub fn in_typing_mode(&self) -> bool {
		self.input_state.is_some()
	}

	/// If `in_typing_mode()` returns true, this should be called with incoming typing events, and
	/// also the `world_header` instance because it for some actions.
	/// Returns `BoardMessage`s if the input is accepted (eg. when you press enter in the save game
	/// name input box)
	pub fn process_typing(&mut self, event: TypingEvent, world_header: &WorldHeader) -> Vec<BoardMessage> {
		let mut board_messages = vec![];

		let mut close_input = false;

		if let Some(ref mut input_state) = self.input_state {
			match input_state {
				InputState::Text(ref mut text_input_state) => {
					match event {
						TypingEvent::Char(mut c) => {
							if text_input_state.mode.max_char_len() > text_input_state.text.len() {
								if text_input_state.mode.force_upper() {
									c.make_ascii_uppercase();
								}

								text_input_state.text.data.push(c);
							}
						}
						TypingEvent::Escape => {
							close_input = true;
						}
						TypingEvent::Enter => {
							match text_input_state.mode {
								TextInputMode::SaveFile => {
									let mut file_name = std::mem::replace(&mut text_input_state.text, DosString::new());
									file_name += text_input_state.mode.suffix();
									board_messages.push(BoardMessage::SaveGameToFile(file_name));
								}
								TextInputMode::Debug => {
									board_messages.push(BoardMessage::DebugCommand(std::mem::replace(&mut text_input_state.text, DosString::new())));
								}
							}
							close_input = true;
						}
						TypingEvent::Backspace => {
							text_input_state.text.data.pop();
						}
						_ => {}
					}
				}
				InputState::YesNo(mode) => {
					match event {
						TypingEvent::Char(b'y') | TypingEvent::Char(b'Y') => {
							match mode {
								YesNoMode::EndGame => {
									board_messages.push(BoardMessage::ReturnToTitleScreen);
									let mut filename = world_header.world_name.clone();
									filename = filename.to_upper();
									filename += b".ZZT";
									board_messages.push(BoardMessage::OpenWorld{filename});
								}
								YesNoMode::Quit => {
									board_messages.push(BoardMessage::Quit);
								}
							}
							close_input = true;
						}
						TypingEvent::Escape | TypingEvent::Char(b'n') | TypingEvent::Char(b'N') => {
							close_input = true;
						}
						_ => {}
					}
				}
			}

		}

		if close_input {
			self.input_state = None;
		}

		board_messages
	}

	/// Open a text input box in the side bar with the given `mode`. See `TextInputMode`.
	/// The text input will start with the given `default` text.
	pub fn open_text_input(&mut self, mode: TextInputMode, default: &[u8]) {
		self.input_state = Some(InputState::Text(TextInputState {
			mode,
			text: DosString::from_slice(default),
		}));
	}

	/// Open a yes/no confirmation input in the side bar, with the given `mode`. See `YesNoMode`.
	pub fn open_yes_no_input(&mut self, mode: YesNoMode) {
		self.input_state = Some(InputState::YesNo(mode));
	}

	/// Draw `num` as a decimal number at the given `x`/`y` position in the console, with the given
	/// `foreground` and `background` colours on each character that is drawn.
	fn draw_num_at(&self, x: usize, y: usize, num: isize, background: ConsoleColour, foreground: ConsoleColour, console_state: &mut ConsoleState) {
		console_state.draw_text_at(x, y, num.to_string().as_bytes(), background, foreground);
	}

	/// Draw a "hotkey" at the given `x`/`y` position in the console. These are displayed in the
	/// side bar as the `key` that needs to be pressed to invoke a certain action, with the name of
	/// the action to the right (`description`). If `gray_key` is true, use a gray background behind
	/// the `key`, otherwise use an aqua background. If `yellow` is true, use yellow text for the
	/// `description`, otherwise use white text.
	fn draw_hotkey(&self, x: usize, y: usize, key: &[u8], description: &[u8], gray_key: bool, yellow: bool, console_state: &mut ConsoleState) {
		let key_back = if gray_key {ConsoleColour::LightGray} else {ConsoleColour::Cyan};
		let desc_fore = if yellow {ConsoleColour::Yellow} else {ConsoleColour::White};

		console_state.draw_text_at(x, y, key, key_back, ConsoleColour::Black);
		console_state.draw_text_at(x + key.len() + 1, y, description, ConsoleColour::Blue, desc_fore);
	}

	/// Draw the blue background of the side bar.
	fn draw_background(&self, console_state: &mut ConsoleState) {
		for y in 0..25 {
			for x in 60..80 {
				*console_state.get_char_mut(x, y) = ConsoleChar::new(0, ConsoleColour::Blue, ConsoleColour::Black);
			}
		}
	}

	/// If an input is open in the side bar, this is used to render it. The `cycle` is used for
	/// yes/no inputs to make the underscore blink.
	fn draw_input(&self, console_state: &mut ConsoleState, cycle: usize) {
		use self::ConsoleColour::*;

		if let Some(ref input_state) = self.input_state {
			match input_state {
				InputState::Text(ref text_input_state) => {
					let mut text = text_input_state.text.clone();
					let max_char_len = text_input_state.mode.max_char_len();
					for _ in 0 .. (max_char_len - text.len()) {
						text.push(b' ');
					}
					for c in text_input_state.mode.suffix() {
						text.push(*c);
					}

					console_state.draw_text_at(63, 5, &text.data, Black, White);
					*console_state.get_char_mut(63 + text_input_state.text.len(), 4) = ConsoleChar::new(0x1f, Blue, White);
				}
				InputState::YesNo(ref mode) => {
					let message: &[u8] = match mode {
						YesNoMode::EndGame => b"End this game?",
						YesNoMode::Quit => b"Quit RUZZT?",
					};
					console_state.draw_text_at(63, 5, message, Blue, White);
					if cycle % 6 < 3 {
						*console_state.get_char_mut(63 + message.len() + 1, 5) = ConsoleChar::new(0x5f, Blue, White);
					}
				}
			}
		}
	}

	/// Draw the side bar in the console.
	pub fn draw_side_bar(&self, world_header: &WorldHeader, current_board_meta_data: &BoardMetaData, is_paused: bool, in_title_screen: bool, console_state: &mut ConsoleState, cycle: usize) {
		use self::ConsoleColour::*;

		self.draw_background(console_state);
		console_state.draw_text_at(65, 0, b"- - - - -", Blue, White);
		console_state.draw_text_at(62, 1, b"     RUZZT     ", LightGray, Black);
		console_state.draw_text_at(65, 2, b"- - - - -", Blue, White);

		if in_title_screen {
			self.draw_title_content(world_header, console_state);
		} else {
			self.draw_game_content(world_header, current_board_meta_data, is_paused, console_state);
		}

		self.draw_input(console_state, cycle);
	}

	/// Draw the side bar in the title screen mode.
	fn draw_title_content(&self, world_header: &WorldHeader, console_state: &mut ConsoleState) {
		use self::ConsoleColour::*;

		if self.input_state.is_none() {
			console_state.draw_text_at(62, 5, b"Pick a command:", Blue, LightCyan);
		}

		self.draw_hotkey(62, 07, b" W ", b"World:", false, true, console_state);
		let world_name = if world_header.world_name.len() == 0 {
			b"Untitled"
		} else {
			world_header.world_name.data.as_slice()
		};
		console_state.draw_text_at(69, 08, world_name, Blue, White);

		self.draw_hotkey(62, 11, b" P ", b"Play", true, false, console_state);
		self.draw_hotkey(62, 12, b" R ", b"Restore game", false, true, console_state);
		self.draw_hotkey(62, 13, b" Q ", b"Quit", true, true, console_state);

		//self.draw_hotkey(62, 16, b" A ", b"About ZZT!", false, false, console_state);
		//self.draw_hotkey(62, 17, b" H ", b"High Scores", true, true, console_state);
		//self.draw_hotkey(62, 18, b" E ", b"Board Editor", false, true, console_state);

		//self.draw_hotkey(62, 20, b" S ", b"Game speed:", true, true, console_state);
	}

	/// Draw the side bar in the in-game mode.
	fn draw_game_content(&self, world_header: &WorldHeader, current_board_meta_data: &BoardMetaData, is_paused: bool, console_state: &mut ConsoleState) {
		use self::ConsoleColour::*;

		if is_paused {
			console_state.draw_text_at(64, 5, b"Pausing...", Blue, White);
		}

		if current_board_meta_data.time_limit > 0 {
			let time_left = current_board_meta_data.time_limit - world_header.time_passed;
			console_state.draw_text_at(64, 06, b"   Time:", Blue, Yellow);
			self.draw_num_at(72, 06, time_left as isize, Blue, Yellow, console_state);
		}

		*console_state.get_char_mut(62, 07) = ConsoleChar::new(0x02, Blue, White);
		console_state.draw_text_at(64, 07, b" Health:", Blue, Yellow);
		self.draw_num_at(72, 07, world_header.player_health as isize, Blue, Yellow, console_state);

		*console_state.get_char_mut(62, 08) = ConsoleChar::new(0x84, Blue, LightCyan);
		console_state.draw_text_at(64, 08, b"   Ammo:", Blue, Yellow);
		self.draw_num_at(72, 08, world_header.player_ammo as isize, Blue, Yellow, console_state);

		if let Some(player_torches) = world_header.player_torches {
			*console_state.get_char_mut(62, 09) = ConsoleChar::new(0x9D, Blue, Brown);
			console_state.draw_text_at(64, 09, b"Torches:", Blue, Yellow);
			self.draw_num_at(72, 09, player_torches as isize, Blue, Yellow, console_state);
		}

		if let Some(torch_cycles) = world_header.torch_cycles {
			if torch_cycles != 0 {
				for i in 0..4 {
					let char_code = if i < torch_cycles / 40 {
						0xb1
					} else {
						0xb0
					};
					*console_state.get_char_mut(75 + i as usize, 09) = ConsoleChar::new(char_code, Blue, Brown);
				}
			}
		}

		*console_state.get_char_mut(62, 10) = ConsoleChar::new(0x04, Blue, LightCyan);
		console_state.draw_text_at(64, 10, b"   Gems:", Blue, Yellow);
		self.draw_num_at(72, 10, world_header.player_gems as isize, Blue, Yellow, console_state);

		console_state.draw_text_at(64, 11, b"  Score:", Blue, Yellow);
		self.draw_num_at(72, 11, world_header.player_score as isize, Blue, Yellow, console_state);

		*console_state.get_char_mut(62, 12) = ConsoleChar::new(0x0C, Blue, White);
		console_state.draw_text_at(64, 12, b"   Keys:", Blue, Yellow);
		for i in 0 .. 7 {
			if world_header.player_keys[i] {
				*console_state.get_char_mut(72 + i, 12) = ConsoleChar::new(0x0C, Blue, ConsoleColour::from_u8(i as u8 + 9).unwrap());
			}
		}

		self.draw_hotkey(62, 14, b" T ", b"Torch", true, false, console_state);
		self.draw_hotkey(62, 15, b" B ", b"Be quiet", false, false, console_state);
		self.draw_hotkey(62, 16, b" H ", b"Help", true, false, console_state);

		self.draw_hotkey(67, 18, b" \x18\x19\x1A\x1B", b"Move", false, false, console_state);
		self.draw_hotkey(61, 19, b" Shift \x18\x19\x1A\x1B", b"Shoot", true, false, console_state);

		self.draw_hotkey(62, 21, b" S ", b"Save game", true, false, console_state);
		self.draw_hotkey(62, 22, b" P ", b"Pause", false, false, console_state);
		self.draw_hotkey(62, 23, b" Q ", b"Quit", true, false, console_state);
	}
}
