use crate::event::*;
use crate::board_message::*;
use crate::console::*;
use zzt_file_format::dosstring::DosString;

/// A scroll has a few horizontal borders in it that are all drawn in a similar manner. This
/// represents the type of border to draw.
#[derive(PartialEq)]
enum ScrollBorder {
	/// The border at the top of the scroll.
	Top,
	/// The border between the title and the content.
	UnderHeading,
	/// The border around lines of text (the two vertical lines on the left and right of each line
	/// of text in the scroll's content and title).
	Text,
	/// The border at the bottom of the scroll.
	Bottom,
}

/// Represents the different ways that lines of text are formatted when drawing the scroll.
enum ScrollTextRowType {
	// Yellow, 2 char indent
	Normal,
	/// Centred, yellow
	Title,
	/// Centred, white
	Centred,
	/// White text with a pink arrow on the left.
	Link,
	// Yellow, no indent
	Yellow,
}

/// The current state of a scroll.
#[derive(Clone)]
pub struct ScrollState {
	/// The title of the scroll, displayed persistently at the top.
	title: DosString,
	/// The lines of text in the scroll content area.
	content_lines: Vec<DosString>,
	/// The line index within `content_lines` that the user currently has selected.
	current_line: isize,
}

impl ScrollState {
	/// Make a new scroll state with the given `title` and `content_lines`.
	pub fn new_title_content(title: DosString, content_lines: Vec<DosString>) -> ScrollState {
		ScrollState {
			title,
			content_lines,
			current_line: 0,
		}
	}

	/// Make a new scroll state with no title and no content.
	pub fn new_empty() -> ScrollState {
		ScrollState {
			title: DosString::new(),
			content_lines: vec![],
			current_line: 0,
		}
	}

	/// If the current line represents a link, this will return the target string for that link.
	/// For example, if the line of text is `!thing;Hello!`, this will return "thing".
	fn hovering_link(&self) -> Option<&[u8]> {
		if self.current_line >= 0 {
			let content_line = &self.content_lines[self.current_line as usize];

			if content_line.get(0) == Some(&b'!') {
				for (i, char_code) in content_line[1..].iter().enumerate() {
					if *char_code == b';' {
						return Some(&content_line[1..i + 1]);
					}
				}
				// When there is no ; in the line, the link still works, but the text shown is the
				// whole line including the !
				Some(&content_line[1..])
			} else {
				None
			}
		} else {
			None
		}
	}

	/// Execute a single simulation step on the scroll, with the given input `event`.
	pub fn step(&mut self, event: Event) -> Vec<BoardMessage> {
		let mut board_messages = vec![];
		let page_size = 14;
		match event {
			Event::Escape => {
				board_messages.push(BoardMessage::CloseScroll);
			}
			Event::Enter => {
				if let Some(hovering_link_label) = self.hovering_link() {
					let label = DosString::from_slice(hovering_link_label);
					board_messages.push(BoardMessage::LinkClicked(label));
					board_messages.push(BoardMessage::CloseScroll);
				} else {
					board_messages.push(BoardMessage::CloseScroll);
				}
				board_messages.push(BoardMessage::EnterPressedInScroll{line_index: self.current_line as usize});
			}
			Event::Up => {
				if self.current_line > 0 {
					self.current_line -= 1;
				}
			}
			Event::Down => {
				if self.current_line < self.content_lines.len() as isize - 1 {
					self.current_line += 1;
				}
			}
			Event::PageUp => {
				self.current_line -= page_size;
				if self.current_line < 0 {
					self.current_line = 0;
				}
			}
			Event::PageDown => {
				self.current_line += page_size;
				if self.current_line > self.content_lines.len() as isize - 1 {
					self.current_line = self.content_lines.len() as isize - 1;
				}
			}
			_ => {}
		}
		board_messages
	}

	/// Draws a horizontal scroll border with the given `mode`, on the given `row` in the console.
	fn draw_border(&self, row: usize, mode: ScrollBorder, console_state: &mut ConsoleState) {
		let chars = match mode {
			ScrollBorder::Top => (0xc6, 0xd1, 0xcd, 0xd1, 0xb5),
			ScrollBorder::UnderHeading => (0, 0xc6, 0xcd, 0xb5, 0),
			ScrollBorder::Text => (0, 0xb3, 0, 0xb3, 0),
			ScrollBorder::Bottom => (0xc6, 0xcf, 0xcd, 0xcf, 0xb5),
		};

		let bg = ConsoleColour::Black;
		let fg = ConsoleColour::White;

		*console_state.get_char_mut(5, row) = ConsoleChar::new(chars.0, bg, fg);
		*console_state.get_char_mut(6, row) = ConsoleChar::new(chars.1, bg, fg);
		for x in 7 ..= 51 {
			*console_state.get_char_mut(x, row) = ConsoleChar::new(chars.2, bg, fg);
		}
		*console_state.get_char_mut(52, row) = ConsoleChar::new(chars.3, bg, fg);
		*console_state.get_char_mut(53, row) = ConsoleChar::new(chars.4, bg, fg);
	}

	/// Draw the given `text` in the console, starting at the given `x`/`y` position, with a blue
	/// background, and the given `foreground` colour.
	fn draw_text_at(&self, x: usize, y: usize, text: &[u8], foreground: ConsoleColour, console_state: &mut ConsoleState) {
		for (i, char_code) in text.iter().enumerate() {
			*console_state.get_char_mut(x + i, y) = ConsoleChar::new(*char_code, ConsoleColour::Blue, foreground);
		}
	}

	/// Renders the given `text`to the given `row` in the console, formatted as appropriate for the
	/// given `mode`.
	fn draw_text_row(&self, row: usize, text: &[u8], mode: ScrollTextRowType, console_state: &mut ConsoleState) {
		let bg = ConsoleColour::Blue;
		let fg = match mode {
			ScrollTextRowType::Normal | ScrollTextRowType::Title | ScrollTextRowType::Yellow => ConsoleColour::Yellow,
			ScrollTextRowType::Centred | ScrollTextRowType::Link => ConsoleColour::White,
		};

		let left_x = 7;
		let total_width = 45;
		for x in 0 .. total_width {
			let char_code = if row == 13 {
				if x == 0 {
					0xaf
				} else if x == total_width - 1 {
					0xae
				} else {
					0
				}
			} else {
				0
			};
			*console_state.get_char_mut(left_x + x, row) = ConsoleChar::new(char_code, bg, ConsoleColour::LightRed);
		}

		let total_content_width = 42;

		match mode {
			ScrollTextRowType::Title | ScrollTextRowType::Centred => {
				let start_x = left_x + 2 + (total_content_width / 2) - ((text.len() + 1) / 2);
				self.draw_text_at(start_x, row, text, fg, console_state);
			}
			ScrollTextRowType::Normal => {
				self.draw_text_at(left_x + 2, row, text, fg, console_state);
			}
			ScrollTextRowType::Link => {
				self.draw_text_at(left_x + 4, row, &[0x10], ConsoleColour::LightMagenta, console_state);
				self.draw_text_at(left_x + 7, row, text, fg, console_state);
			}
			ScrollTextRowType::Yellow => {
				self.draw_text_at(left_x, row, text, fg, console_state);
			}
		};
	}

	/// Renders all the different borders of the scroll to the console.
	fn draw_all_borders(&self, console_state: &mut ConsoleState) {
		self.draw_border(3, ScrollBorder::Top, console_state);
		self.draw_border(4, ScrollBorder::Text, console_state);
		self.draw_border(5, ScrollBorder::UnderHeading, console_state);
		for row in 6 ..= 20 {
			self.draw_border(row, ScrollBorder::Text, console_state);
		}
		self.draw_border(21, ScrollBorder::Bottom, console_state);
	}

	/// Renders the scroll and all of its contents.
	pub fn draw_scroll(&self, console_state: &mut ConsoleState) {
		// When ZZT draws a scroll, it first animates in just the borders (filled with black).
		// When the animation finishes, it goes through each line and sets out the blue background
		// empty chars from left to right (this is also where it puts in the little red arrows).
		// Then it sets the text and foreground colours from left to right.
		// When drawing the "Use up down, enter to view text", it draws all the green text first,
		// left to right, then draws the white text.
		self.draw_all_borders(console_state);
		let title = if self.hovering_link().is_some() {
			b"\xAEPress ENTER to select this\xAF"
		} else {
			self.title.data.as_slice()
		};

		self.draw_text_row(4, title, ScrollTextRowType::Title, console_state);

		for row in 6 ..= 20 {
			let content_line_index = row - 13 + self.current_line;

			let in_bounds = content_line_index >= 0 && content_line_index < self.content_lines.len() as isize;
			let (line_text, line_type): (&[u8], ScrollTextRowType) = if in_bounds {
				let content_line = &self.content_lines[content_line_index as usize].data;
				if content_line.len() >= 1 {
					match content_line[0] {
						b'$' => (&content_line[1..], ScrollTextRowType::Centred),
						b'!' => {
							// The default is 0, so when there is no ; it draws text from the !
							// onwards.
							let mut link_text_start = 0;
							for (i, char_code) in content_line.iter().enumerate() {
								if *char_code == b';' {
									link_text_start = i + 1;
									break;
								}
							}
							(&content_line[link_text_start..], ScrollTextRowType::Link)
						}
						_ => (content_line, ScrollTextRowType::Normal)
					}
				} else {
					(content_line, ScrollTextRowType::Normal)
				}
			} else if content_line_index == -1 || content_line_index == self.content_lines.len() as isize {
				(b"    \x07    \x07    \x07    \x07    \x07    \x07    \x07    \x07    \x07",
					ScrollTextRowType::Yellow)
			} else {
				(b"", ScrollTextRowType::Normal)
			};

			self.draw_text_row(row as usize, line_text, line_type, console_state);
		}
	}
}
