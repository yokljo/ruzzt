use crate::behaviour::*;
use crate::board_message::*;
use crate::board_simulator::*;
use crate::direction::*;
use crate::console::ConsoleColour;
use crate::sounds::*;

use zzt_file_format::*;
use zzt_file_format::dosstring::DosString;

use rand::{self, Rng};
use num::FromPrimitive;
use std::borrow::Cow;
use std;

/// Describes a particular OOP operator, which is always determined by the value of the first
/// character on a line.
#[derive(Debug, PartialEq)]
pub enum OopOperator {
	/// "@"
	Name,
	/// "/"
	Move,
	/// "?"
	TryMove,
	/// ":"
	Label,
	/// "'"
	Comment,
	/// "#"
	Command,
	/// A "say text" command, which will result in a line of text being added to a list. When the
	/// script finishes executing, that list is used to open a scroll, or if there is only one line
	/// of text, it will just be displayed as a caption at the bottom of the screen.
	/// This includes lines that start with ; or $ because they are actually handled by the scroll.
	Text,
	/// End of file.
	Eof,
}

/// Call `found_fn` for each tile on the board matching the given `tile_desc`.
/// `found_fn` takes the x/y position of each matching tile.
fn search_tile_desc(tile_desc: TileTypeDesc, sim: &BoardSimulator, found_fn: &mut FnMut(i16, i16, BoardTile)) {
	sim.visit_all_tiles(&mut |x, y, tile| {
		if tile_desc.matches(tile) {
			found_fn(x, y, tile);
		}
	});
}

/// Create an `Action` that spawns a tile matching the given `tile_desc` at the given `x`/`y`
/// position on the board.
fn create_tile_action(tile_desc: &TileTypeDesc, x: u8, y: u8) -> Action {
	let colour = tile_desc.colour.unwrap_or(0x0f);

	let tile = BoardTile {
		element_id: tile_desc.element_id,
		colour,
	};

	let mut status_element = None;

	if let Some(ty) = ElementType::from_u8(tile_desc.element_id) {
		match ty {
			| ElementType::Bear
			| ElementType::BlinkWall
			| ElementType::Bomb
			| ElementType::Bullet
			| ElementType::Clockwise
			| ElementType::Counter
			| ElementType::Duplicator
			| ElementType::Head
			| ElementType::Lion
			| ElementType::Object
			| ElementType::Passage
			| ElementType::Pusher
			| ElementType::Ruffian
			| ElementType::Scroll
			| ElementType::Segment
			| ElementType::Shark
			| ElementType::Slime
			| ElementType::SpinningGun
			| ElementType::Tiger
			| ElementType::Transporter
			=> {
				status_element = Some(StatusElement {
					location_x: x,
					location_y: y,
					cycle: 3,
					.. StatusElement::default()
				});
			}
			ElementType::Star => {
				status_element = Some(StatusElement {
					location_x: x,
					location_y: y,
					cycle: 1,
					param2: 255,
					.. StatusElement::default()
				});
			}
			_ => {}
		}
	}

	Action::SetTile{x: x as i16, y: y as i16, tile, status_element}
}

/// A description of a tile to search for or spawn. This is parsed from text in OOP such as
/// `#change blue boulder gem`, which would create two `TileTypeDesc`s, one describing a blue
/// boulder, and another describing a gem with no particular colour.
#[derive(Debug, Copy, Clone)]
struct TileTypeDesc {
	/// The element ID of the tile.
	element_id: u8,
	/// The (optional) colour of the tile.
	colour: Option<u8>,
}

impl TileTypeDesc {
	/// Returns true if the `tile` matches the description.
	fn matches(&self, tile: BoardTile) -> bool {
		let colour_matches = if let Some(colour) = self.colour {
			colour == tile.colour
		} else {
			true
		};

		let element_id_matches = self.element_id == tile.element_id;

		colour_matches && element_id_matches
	}
}

/// A description of the reciever of a message, when the `#send blah` command is used for example.
#[derive(Debug, Clone, PartialEq)]
pub enum ReceiverDesc {
	/// Send the message to the current status.
	Myself,
	/// Send the message to all statuses, including the current status.
	All,
	/// Send the message to all other statuses that aren't the current status.
	Others,
	/// Send the message to the status with a name specified at the start of it's code, like `@Bob`.
	Name(DosString),
}

impl ReceiverDesc {
	/// Get the `ReceiverDesc` associated with the given name. For example `all:bombed` will return
	/// `ReceiverDesc::All`.
	/// This expects name to be lower case.
	fn from_name(name: &[u8]) -> ReceiverDesc {
		match name {
			b"all" => ReceiverDesc::All,
			b"others" => ReceiverDesc::Others,
			name => ReceiverDesc::Name(DosString::from_slice(name)),
		}
	}
}

/// A description of a message to send.
#[derive(Debug, Clone, PartialEq)]
pub struct MessageDesc {
	/// The reciever of the message.
	receiver: ReceiverDesc,
	/// The label to jump to in the reciever's code.
	label: DosString,
}

/// Some commands need to apply an action and then check the result. This stores the type of
/// checking on that result that needs to take place after the action has been applied.
#[derive(Debug)]
enum OopAsyncAction {
	/// Special handling for moving the code_current_instruction to a new location if the result of
	/// an action is NotBlocked. ie. when you parse "/s", it won't move the code position, but when
	/// its move attempt succeeds, this will be set to Some(2) (or whatever the position after the
	/// "/s" happens to be), so it moves the code pos after the "/s".
	Move {
		instruction_when_not_blocked: i16,
	},
	/// Note that this allows for full commands, not just messages (which is what the manual says).
	TryMove,
	/// Note that this allows for full commands, not just messages (which is what the manual says).
	Take,
	/// The put action tries to push something out of the way, and then it checks the type of the
	/// tile. If the type is the same as the type trying to be inserted, it only changes the colour,
	/// otherwise it replaces the whole thing with a new tile, status and all.
	Put {
		direction: Direction,
		tile_type: TileTypeDesc,
	},
}

/// When an OOP action has been parsed, this describes what to do after in the `OopExecutionState`.
#[derive(Debug)]
pub struct ParseActionOutcome {
	finish_immediately: bool,
	dont_progress: bool,
}

/// This is an `ActionContinuation` that is used by Scroll and Object element behaviours to execute
/// their OOP code.
#[derive(Debug)]
pub struct OopExecutionState {
	/// If this is true, the tile with the executing status code will be set to empty.
	delete_after: bool,
	/// The status to actually use. If not set, will use the current executing status.
	override_working_status_index: Option<usize>,
	/// The number of executed OOP operations. After a certain number, the OOP will be forced to
	/// stop running, preventing the game from hanging.
	executed_operation_count: usize,
	/// A single scroll will be created from all the lines of text read in one parsing session.
	/// eg. if you have some text, then something like a #play command that doesn't halt the
	/// program parsing, then more text, those lines of text will be placed into a scroll
	/// regardless of the #play command that's excecuted in the middle.
	/// Since text parsing doesn't stop parsing from happening, the command immediately after the
	/// scroll text will always be excecuted before the scroll appears.
	text_message_content_lines: Vec<DosString>,
	/// Some commands require applying a push action, then checking if it is blocked, and doing
	/// something in response to that. Those commands will set this to Some(blah) so they can check
	/// on the next call to next_step whether they should do anything.
	action_to_check_on_next_step: Option<OopAsyncAction>,
	/// The start of the current action being executed, at the # or / character.
	current_start_of_action_pos: Option<i16>,
}

impl OopExecutionState {
	pub fn new(delete_after: bool, override_working_status_index: Option<usize>) -> OopExecutionState {
		OopExecutionState {
			delete_after,
			override_working_status_index,
			executed_operation_count: 0,
			text_message_content_lines: vec![],
			action_to_check_on_next_step: None,
			current_start_of_action_pos: None,
		}
	}

	/// Returns true if the current execution should finish.
	fn apply_outcome_result(&mut self, outcome_result: Result<ParseActionOutcome, DosString>, parser: &mut OopParser, actions: &mut Vec<Action>) -> bool {
		let mut is_finished = false;
		match outcome_result {
			Ok(outcome) => {
				if outcome.dont_progress {
					if let Some(current_start_of_action_pos) = self.current_start_of_action_pos {
						parser.pos = current_start_of_action_pos;
						self.current_start_of_action_pos = None;
					} else {
						panic!("current_start_of_action_pos should not be None");
					}
				}

				if outcome.finish_immediately {
					is_finished = true;
				}
			}
			Err(error_string) => {
				println!("OOP Error: {:?}", error_string);
				actions.push(Action::SendBoardMessage(BoardMessage::OpenScroll {
					title: DosString::new(),
					content_lines: vec![error_string],
				}));
				is_finished = true;
			}
		}
		return is_finished;
	}
}

impl ActionContinuation for OopExecutionState {
	fn next_step(&mut self, apply_action_report: ApplyActionResultReport, status_index: usize, _status: &StatusElement, sim: &BoardSimulator) -> ActionContinuationResult {
		let working_status_index = self.override_working_status_index.unwrap_or(status_index);
		let ref status = sim.status_elements[working_status_index];

		if status.code_current_instruction < 0 {
			// If the code_current_instruction is negative, then the program is not running.
			return ActionContinuationResult {
				actions: vec![],
				finished: true,
			};
		}

		//let mut debug_code = sim.get_status_code(status).clone();
		//debug_code.data.insert(status.code_current_instruction as usize, b'~');
		//debug_code.data.drain(0 .. status.code_current_instruction as usize);
		//println!("{:?}: {:?}", status.code_current_instruction, debug_code);

		let mut is_finished = false;

		let mut parser = OopParser::new(&sim.get_status_code(status), status.code_current_instruction);
		//println!("{:?} {}", parser.get_scroll_title(), working_status_index);

		let mut actions = vec![];

		if let Some(ref async_action) = self.action_to_check_on_next_step {
			match async_action {
				OopAsyncAction::Move{instruction_when_not_blocked} => {
					if apply_action_report.move_was_blocked == BlockedStatus::NotBlocked {
						parser.pos = *instruction_when_not_blocked;
					}
					is_finished = true;
					self.action_to_check_on_next_step = None;
				}
				OopAsyncAction::TryMove => {
					// This needs to be reset before parse_command is called because it might set it
					// to something else.
					self.action_to_check_on_next_step = None;
					if apply_action_report.move_was_blocked == BlockedStatus::Blocked {
						let outcome_result = parser.parse_command(working_status_index, status, &mut actions, self, sim);
						is_finished = self.apply_outcome_result(outcome_result, &mut parser, &mut actions);
					} else {
						parser.read_to_end_of_line();
						parser.skip_new_line();
						is_finished = true;
					}
				}
				OopAsyncAction::Put{direction, tile_type} => {
					let (off_x, off_y) = direction.to_offset();
					let dest_x = status.location_x as i16 + off_x;
					let dest_y = status.location_y as i16 + off_y;
					if let Some(dest_tile) = sim.get_tile(dest_x, dest_y) {
						if dest_tile.element_id == tile_type.element_id {
							if let Some(colour) = tile_type.colour {
								actions.push(Action::SetColour {
									x: dest_x,
									y: dest_y,
									colour,
								});
							} else {
								// There is no colour specified, and the element IDs are the same,
								// so do nothing.
							}
						} else {
							actions.push(create_tile_action(&tile_type, dest_x as u8, dest_y as u8));
						}
					} else {
						// There's no tile at the destination...
					}
					self.action_to_check_on_next_step = None;
				}
				OopAsyncAction::Take => {
					// This needs to be reset before parse_command is called because it might set it
					// to something else.
					self.action_to_check_on_next_step = None;
					// `#take ammo 20 go s` will try to infinitely take 20 ammo until it can
					// either take the 20 ammo, or move south.
					if apply_action_report.take_player_item_failed {
						let outcome_result = parser.parse_command(working_status_index, status, &mut actions, self, sim);
						is_finished = self.apply_outcome_result(outcome_result, &mut parser, &mut actions);
					} else {
						parser.read_to_end_of_line();
						parser.skip_new_line();
					}
				}
			}
		} else {
			// Before parse_action is called, save the parser position, because that is the
			// position that needs to be jumped back to when something like `/s` or `#go s` is
			// called and the move fails. This is important because eg. `#take ammo 20 go s` results
			// in nested asynchronous actions, where when either the take OR the go fail, it jumps
			// all the way back to the start of the `#take`.
			self.current_start_of_action_pos = Some(parser.pos);

			let outcome_result = parser.parse_action(working_status_index, status, &mut actions, self, sim);
			is_finished = self.apply_outcome_result(outcome_result, &mut parser, &mut actions);
		}

		self.executed_operation_count += 1;

		// ZZT will excecute a maximum of 64 "instructions" (basically 64 lines of code, except for
		// stuff like /s/s/s/s).
		if self.executed_operation_count > 64 {
			is_finished = true;
		}

		if parser.pos != status.code_current_instruction {
			if parser.pos > status.code_current_instruction {
				//let mut code = sim.get_status_code(status).clone();
				//let exec_code = DosString::from_slice(&code[status.code_current_instruction as usize .. parser.pos as usize]);
				//println!("exec: {:?}", exec_code);
			}
			// Insert at 0 so it changes the code instruction before doing anything else.
			actions.insert(0, Action::SetCodeCurrentInstruction{status_index: working_status_index, code_current_instruction: parser.pos});
		}

		if let Cow::Owned(new_code) = parser.code {
			actions.push(Action::SetCode{status_index: working_status_index, code: new_code});
		}

		ActionContinuationResult {
			actions,
			finished: is_finished,
		}
	}

	fn finalise(&mut self, status_opt: Option<&StatusElement>, sim: &BoardSimulator) -> Vec<Action> {
		let mut actions = vec![];

		if self.text_message_content_lines.len() > 0 {
			println!("{:?}", self.text_message_content_lines);
			let title = {
				if let Some(status) = status_opt {
					let parser = OopParser::new(&sim.get_status_code(status), status.code_current_instruction);
					parser.get_scroll_title().unwrap_or_else(|| DosString::from_slice(b"Interaction"))
				} else {
					DosString::from_slice(b"Interaction")
				}
			};

			actions.push(Action::SendBoardMessage(BoardMessage::OpenScroll {
				title,
				content_lines: std::mem::replace(&mut self.text_message_content_lines, vec![]),
			}));
		}

		if self.delete_after {
			if let Some(status_index) = self.override_working_status_index {
				let ref status = sim.status_elements[status_index];
				actions.push(Action::SetTile {
					x: status.location_x as i16,
					y: status.location_y as i16,
					tile: BoardTile { element_id: ElementType::Empty as u8, colour: 0 },
					status_element: None,
				});
			} else {
				if let Some(status) = status_opt {
					actions.push(Action::SetTile {
						x: status.location_x as i16,
						y: status.location_y as i16,
						tile: BoardTile { element_id: ElementType::Empty as u8, colour: 0 },
						status_element: None,
					});
				}
			}
		}

		actions
	}
}

/// This is used to parse OOP code.
pub struct OopParser<'code> {
	/// The code being parsed. Note that this can be modified because of the `#zap` and `#restore`
	/// commands, which change comments to labels and vice versa.
	pub code: Cow<'code, DosString>,
	/// The current position to parse the code from.
	pub pos: i16,
}

impl<'code> OopParser<'code> {
	pub fn new(code: &'code DosString, pos: i16) -> OopParser<'code> {
		// TODO: pos becomes -1 when an error occurs so the program stops running.
		/*let good_pos = if pos >= 0 {
			pos
		} else {
			0
		};*/

		OopParser {
			code: Cow::Borrowed(code),
			pos: pos,
		}
	}

	/// Parse a single action from the OOP code, and modify `actions` and `state` accordingly.
	pub fn parse_action(&mut self, status_index: usize, status: &StatusElement, actions: &mut Vec<Action>, state: &mut OopExecutionState, sim: &BoardSimulator) -> Result<ParseActionOutcome, DosString> {
		let mut outcome = ParseActionOutcome {
			finish_immediately: false,
			dont_progress: false,
		};

		let op = self.parse_operator();
		match op {
			OopOperator::Name => {
				let _name = self.read_to_end_of_line();
				self.skip_new_line();
				//println!("@{:?}", name);
			}
			OopOperator::Move => {
				match self.parse_direction(status, sim) {
					Ok(direction) => {
						let (offset_x, offset_y) = direction.to_offset();
						actions.push(Action::MoveTile{
							from_x: status.location_x as i16,
							from_y: status.location_y as i16,
							to_x: status.location_x as i16 + offset_x,
							to_y: status.location_y as i16 + offset_y,
							offset_x,
							offset_y,
							check_push: true,
							is_player: false,
						});
					}
					Err(direction_name) => {
						println!("Bad direction: {:?}", direction_name);
					}
				}

				// If it's not blocked after doing MoveOwnTile, then it will jump to the position
				// after the direction that was just parsed.
				state.action_to_check_on_next_step = Some(OopAsyncAction::Move {
					instruction_when_not_blocked: self.pos,
				});

				outcome.dont_progress = true;
			}
			OopOperator::TryMove => {
				match self.parse_direction(status, sim) {
					Ok(direction) => {
						let (offset_x, offset_y) = direction.to_offset();
						actions.push(Action::MoveTile{
							from_x: status.location_x as i16,
							from_y: status.location_y as i16,
							to_x: status.location_x as i16 + offset_x,
							to_y: status.location_y as i16 + offset_y,
							offset_x,
							offset_y,
							check_push: true,
							is_player: false,
						});
					}
					Err(direction_name) => {
						println!("Bad direction: {:?}", direction_name);
					}
				}

				outcome.finish_immediately = true;
			}
			OopOperator::Label | OopOperator::Comment => {
				let _ = self.read_to_end_of_line();
				self.skip_new_line();
			}
			OopOperator::Command => {
				outcome = self.parse_command(status_index, status, actions, state, sim)?;
			}
			OopOperator::Text => {
				let mut line = self.read_to_end_of_line();
				// ZZT ignores new lines unless there is already something in the message.
				if line.len() > 0 || state.text_message_content_lines.len() > 0 {
					println!("Line: {:?}", line);
					// Scrolls in ZZT probably use a 2D array of 50 x something chars.
					line.data.truncate(50);
					state.text_message_content_lines.push(line);
				}
				self.skip_new_line();
				//println!("{}", text.to_string(true));

			}
			OopOperator::Eof => {
				self.pos = -1;
				outcome.finish_immediately = true;
			}
		}

		Ok(outcome)
	}

	pub fn parse_operator(&mut self) -> OopOperator {
		if self.pos as usize >= self.code.len() {
			return OopOperator::Eof;
		} else if self.pos as usize == self.code.len() - 1 && self.code.data[self.pos as usize] == b'\r' {
			// ZZT treats the very last new line character in a script as the end of the script.
			return OopOperator::Eof;
		}

		let op_char = self.code.data[self.pos as usize];
		let res = match op_char {
			b'@' => OopOperator::Name,
			b'/' => OopOperator::Move,
			b'?' => OopOperator::TryMove,
			b':' => OopOperator::Label,
			b'\'' => OopOperator::Comment,
			b'#' => OopOperator::Command,
			_ => {
				// Immediately return so pos isn't incremented.
				return OopOperator::Text;
			}
		};
		self.pos += 1;
		res
	}

	fn read_to_end_of_line(&mut self) -> DosString {
		let start_pos = self.pos;
		while self.pos < self.code.data.len() as i16 && self.code.data[self.pos as usize] != 13 {
			self.pos += 1;
		}
		return DosString::from_slice(&self.code.data[start_pos as usize .. self.pos as usize]);
	}

	fn read_word(&mut self) -> DosString {
		let start_pos = self.pos;
		let mut is_first = true;
		while self.pos < self.code.data.len() as i16 {
			let c = self.code.data[self.pos as usize];
			if !is_first && (c >= b'0' && c <= b'9') {
				self.pos += 1;
			} else if (c >= b'A' && c <= b'Z') || (c >= b'a' && c <= b'z') {
				self.pos += 1;
			} else {
				break;
			}
			is_first = false;
		}
		return DosString::from_slice(&self.code.data[start_pos as usize .. self.pos as usize]);
	}

	fn skip_new_line(&mut self) {
		if let Some(c) = self.code.data.get(self.pos as usize) {
			if *c == b'\r' {
				self.pos += 1;
			}
		}
	}

	fn skip_spaces(&mut self) {
		loop {
			if let Some(c) = self.code.data.get(self.pos as usize) {
				if *c == b' ' {
					self.pos += 1;
				} else {
					break;
				}
			} else {
				break;
			}
		}
	}

	pub fn parse_message(&mut self) -> MessageDesc {
		let mut receiver = ReceiverDesc::Myself;
		self.skip_spaces();
		let mut label = self.read_word().to_lower();

		if let Some(b':') = self.code.data.get(self.pos as usize) {
			self.pos += 1;
			receiver = ReceiverDesc::from_name(&label);
			label = self.read_word().to_lower();
		}

		MessageDesc {
			receiver,
			label,
		}
	}

	pub fn parse_direction(&mut self, status: &StatusElement, sim: &BoardSimulator) -> Result<Direction, DosString> {
		let word = self.read_word().to_lower();

		Ok(match word.data.as_slice() {
			b"e" | b"east" => {
				Direction::East
			}
			b"w" | b"west" => {
				Direction::West
			}
			b"n" | b"north" => {
				Direction::North
			}
			b"s" | b"south" => {
				Direction::South
			}
			b"i" | b"idle" => {
				Direction::Idle
			}
			b"seek" => {
				sim.seek_direction(status.location_x as i16, status.location_y as i16)
			}
			b"cw" => {
				self.skip_spaces();
				let direction = self.parse_direction(status, sim)?;
				direction.cw()
			}
			b"ccw" => {
				self.skip_spaces();
				let direction = self.parse_direction(status, sim)?;
				direction.ccw()
			}
			b"flow" => {
				let flow_direction = Direction::from_offset(status.step_x, status.step_y);
				flow_direction
			}
			b"opp" => {
				self.skip_spaces();
				let direction = self.parse_direction(status, sim)?;
				direction.opposite()
			}
			b"rndne" => {
				let mut rng = rand::thread_rng();
				let random_bool: bool = rng.gen();
				if random_bool {
					Direction::North
				} else {
					Direction::East
				}
			}
			b"rndns" => {
				let mut rng = rand::thread_rng();
				let random_bool: bool = rng.gen();
				if random_bool {
					Direction::North
				} else {
					Direction::South
				}
			}
			b"rndp" => {
				self.skip_spaces();
				let direction = self.parse_direction(status, sim)?;
				let mut rng = rand::thread_rng();
				let random_bool: bool = rng.gen();
				match direction {
					Direction::North | Direction::South => {
						if random_bool {
							Direction::East
						} else {
							Direction::West
						}
					}
					Direction::East | Direction::West => {
						if random_bool {
							Direction::North
						} else {
							Direction::South
						}
					}
					_ => Direction::Idle,
				}
			}
			_ => {
				// TODO: "Bad direction: dir"
				return Err(word);
			}
		})
	}

	fn parse_colour(&mut self) -> Result<ConsoleColour, DosString> {
		let word = self.read_word().to_lower();

		Ok(match word.data.as_slice() {
			b"blue" => ConsoleColour::LightBlue,
			b"green" => ConsoleColour::LightGreen,
			b"cyan" => ConsoleColour::LightCyan,
			b"red" => ConsoleColour::LightRed,
			b"purple" => ConsoleColour::LightMagenta,
			b"yellow" => ConsoleColour::Yellow,
			b"white" => ConsoleColour::White,
			_ => {
				// TODO: "Bad colour: blue"
				return Err(word);
			}
		})
	}

	fn parse_type(&mut self) -> Result<ElementType, DosString> {
		let word = self.read_word().to_lower();

		let element_type = match word.data.as_slice() {
			b"ammo" => ElementType::Ammo,
			b"bear" => ElementType::Bear,
			b"blinkwall" => ElementType::BlinkWall,
			b"bomb" => ElementType::Bomb,
			b"boulder" => ElementType::Boulder,
			b"breakable" => ElementType::Breakable,
			b"bullet" => ElementType::Bullet,
			b"clockwise" => ElementType::Clockwise,
			b"counter" => ElementType::Counter,
			b"door" => ElementType::Door,
			b"duplicator" => ElementType::Duplicator,
			b"empty" => ElementType::Empty,
			b"energizer" => ElementType::Energizer,
			b"fake" => ElementType::Fake,
			b"forest" => ElementType::Forest,
			b"gem" => ElementType::Gem,
			b"head" => ElementType::Head,
			b"invisible" => ElementType::Invisible,
			b"key" => ElementType::Key,
			b"line" => ElementType::Line,
			b"lion" => ElementType::Lion,
			b"normal" => ElementType::Normal,
			b"object" => ElementType::Object,
			b"passage" => ElementType::Passage,
			b"pusher" => ElementType::Pusher,
			b"ricochet" => ElementType::Ricochet,
			b"ruffian" => ElementType::Ruffian,
			b"scroll" => ElementType::Scroll,
			b"segment" => ElementType::Segment,
			b"shark" => ElementType::Shark,
			b"sliderew" => ElementType::SliderEW,
			b"sliderns" => ElementType::SliderNS,
			b"slime" => ElementType::Slime,
			b"solid" => ElementType::Solid,
			b"spinninggun" => ElementType::SpinningGun,
			b"star" => ElementType::Star,
			b"tiger" => ElementType::Tiger,
			b"torch" => ElementType::Torch,
			b"transporter" => ElementType::Transporter,
			b"water" => ElementType::Water,
			_ => {
				// TODO: "Bad colour: blue"
				return Err(word);
			}
		};

		Ok(element_type)
	}

	fn parse_player_item(&mut self) -> Result<PlayerItemType, DosString> {
		let word = self.read_word().to_lower();
		let player_item_type = match word.data.as_slice() {
			b"ammo" => PlayerItemType::Ammo,
			b"torches" => PlayerItemType::Torches,
			b"gems" => PlayerItemType::Gems,
			b"health" => PlayerItemType::Health,
			b"score" => PlayerItemType::Score,
			b"time" => PlayerItemType::Time,
			_ => {
				let mut error_msg = DosString::from_slice(b"Bad item: ");
				error_msg += &word.data;
				return Err(error_msg);
			}
		};

		Ok(player_item_type)
	}

	fn parse_tile_type_desc(&mut self) -> Result<TileTypeDesc, DosString> {
		let pos_before_colour = self.pos;

		let colour = match self.parse_colour() {
			Ok(colour) => {
				self.skip_spaces();
				Some(colour as u8)
			}
			Err(_) => {
				self.pos = pos_before_colour;
				None
			}
		};

		let element_id = match self.parse_type() {
			Ok(ty) => ty as u8,
			Err(err) => return Err(err),
		};

		Ok(TileTypeDesc {
			element_id,
			colour,
		})
	}

	fn parse_if_predicate(&mut self, status: &StatusElement, sim: &BoardSimulator) -> Result<bool, DosString> {
		self.skip_spaces();
		let word = self.read_word().to_lower();
		match word.data.as_slice() {
			b"alligned" => {
				// Really good spelling of "aligned" in ZZT lol.
				let (player_x, player_y) = sim.get_player_location();
				Ok(status.location_x as i16 == player_x || status.location_y as i16 == player_y)
			}
			b"any" => {
				self.skip_spaces();
				let find_desc = self.parse_tile_type_desc()?;
				let mut found_any = false;
				for tile in &sim.tiles {
					if find_desc.matches(*tile) {
						found_any = true;
						break;
					}
				}
				Ok(found_any)
			}
			b"blocked" => {
				self.skip_spaces();
				let direction = self.parse_direction(status, sim)?;
				let (off_x, off_y) = direction.to_offset();
				let dest_behaviour = sim.behaviour_for_pos(status.location_x as i16 + off_x, status.location_y as i16 + off_y);
				Ok(dest_behaviour.blocked(false) == BlockedStatus::Blocked)
			}
			b"contact" => {
				let (player_x, player_y) = sim.get_player_location();
				let off_x = (status.location_x as i16 - player_x).abs();
				let off_y = (status.location_y as i16 - player_y).abs();
				Ok((off_x == 0 && off_y == 1) || (off_x == 1 && off_y == 0))
			}
			b"energized" => {
				Ok(sim.world_header.energy_cycles > 0)
			}
			b"not" => {
				Ok(!self.parse_if_predicate(status, sim)?)
			}
			flag_name => {
				// TODO: Unnecessary DosString creation here.
				let flag_is_set = sim.world_header.last_matching_flag(DosString::from_slice(flag_name)).is_some();
				Ok(flag_is_set)
			}
		}
	}

	fn apply_message_desc_label_operation(&mut self, message_desc: MessageDesc, label_op: LabelOperation, status_index: usize, actions: &mut Vec<Action>) {
		let mut includes_myself = false;
		let mut includes_others = false;

		let mut receiver_name_opt = None;

		match message_desc.receiver {
			ReceiverDesc::Myself => {
				includes_myself = true
			}
			ReceiverDesc::All => {
				includes_myself = true;
				includes_others = true;
			}
			ReceiverDesc::Others => {
				includes_others = true;
			}
			ReceiverDesc::Name(receiver_name) => {
				includes_others = true;

				if let Some(my_name) = self.get_name() {
					if receiver_name == my_name {
						includes_myself = true;
					}
				}

				receiver_name_opt = Some(receiver_name);
			}
		}

		if includes_myself {
			self.apply_label_operation(receiver_name_opt.as_ref(), &message_desc.label, label_op);
		}

		if includes_others {
			actions.push(Action::OthersApplyLabelOperation {
				current_status_index: Some(status_index),
				receiver_name_opt,
				label: message_desc.label,
				operation: label_op,
			});
		}
	}

	fn parse_number(&mut self) -> Result<isize, DosString> {
		let start_pos = self.pos;
		let mut num = 0;
		while self.pos < self.code.data.len() as i16 {
			let c = self.code.data[self.pos as usize];
			if c >= b'0' && c <= b'9' {
				let digit_value = (c - b'0') as isize;
				num *= 10;
				num += digit_value;
				self.pos += 1;
			} else {
				break;
			}
		}
		if start_pos == self.pos {
			// I don't think ZZT complains about non-existent numbers, so return empty string.
			Err(DosString::from_slice(b""))
		} else {
			Ok(num)
		}
	}

	/// Returns false if the command should not progress, and the code position should be returned
	/// to the command's # character.
	fn parse_command(&mut self, status_index: usize, status: &StatusElement, actions: &mut Vec<Action>, state: &mut OopExecutionState, sim: &BoardSimulator) -> Result<ParseActionOutcome, DosString> {
		let mut outcome = ParseActionOutcome {
			finish_immediately: false,
			dont_progress: false,
		};

		let message_desc = self.parse_message();

		if message_desc.receiver != ReceiverDesc::Myself {
			// TODO: Should this skip spaces before skipping new line?
			self.skip_new_line();
			self.apply_message_desc_label_operation(message_desc, LabelOperation::Jump, status_index, actions);
		} else {
			// If it's a "myself" message description ("thing", not "blah:thing"), that means
			// there's only one word by itself without a colon, so treat it as the name of the
			// command to execute.
			let command_name = message_desc.label;

			match command_name.data.as_slice() {
				b"" => {
					// A `#` with no valid label following will just skip the hash and parse from
					// there. Eg. "##99" will result in "99" being displayed as a scroll.
				}
				b"become" => {
					// See the comment on #die.
					if state.text_message_content_lines.len() <= 1 {
						self.skip_spaces();
						let become_desc = self.parse_tile_type_desc()?;
						self.read_to_end_of_line();
						self.skip_new_line();
						actions.push(create_tile_action(&become_desc, status.location_x, status.location_y));
						outcome.finish_immediately = true;
					} else {
						// When a scroll is going to open, don't execute the die command just yet,
						// because a link in the scroll might jump elsewhere.
						outcome.finish_immediately = false;
						outcome.dont_progress = true;
					}
				}
				b"bind" => {
					self.skip_spaces();
					let bind_name = self.read_word();
					self.read_to_end_of_line();
					self.skip_new_line();

					let mut bind_to_index_opt = None;
					for status_index in 0 .. sim.status_elements.len() {
						if let CodeSource::Owned(ref code) = sim.status_elements[status_index].code_source {
							let name_parser = OopParser::new(code, 0);
							if let Some(name) = name_parser.get_name() {
								if name == bind_name {
									bind_to_index_opt = Some(status_index);
									break;
								}
							}
						}
					}

					if let Some(bind_to_index) = bind_to_index_opt {
						actions.push(Action::BindCodeToIndex{status_index, bind_to_index});
						actions.push(Action::SetCodeCurrentInstruction{status_index, code_current_instruction: 0});
					}
				}
				b"change" => {
					// Note that `#change object boulder` can replace the current executing object
					// while it is running (unlike with #die or #become), and its script can keep
					// executing after if a scroll is opened and you click a link in it.
					// This also means that change is super unpredicatable because it can result in
					// what seems to be bad pointer accesses, which we are not going to bother
					// replicating because it's basically impossible.
					self.skip_spaces();
					let from_desc = self.parse_tile_type_desc()?;
					self.skip_spaces();
					let mut to_desc = self.parse_tile_type_desc()?;
					self.read_to_end_of_line();
					self.skip_new_line();
					search_tile_desc(from_desc, sim, &mut |x, y, tile| {
						to_desc.colour = Some(tile.colour);
						actions.push(create_tile_action(&to_desc, x as u8, y as u8));
					});
				}
				b"char" => {
					self.skip_spaces();
					if let Ok(char_num) = self.parse_number() {
						if char_num >= 0 && char_num < 256 {
							actions.push(Action::SetStatusParam1{value: char_num as u8, status_index});
						}
					}
					self.read_to_end_of_line();
					self.skip_new_line();
				}
				b"clear" => {
					self.skip_spaces();
					let flag_name = self.read_word();
					self.read_to_end_of_line();
					self.skip_new_line();
					//println!("#clear {:?}", flag_name);
					actions.push(Action::ClearFlag(flag_name));
				}
				b"cycle" => {
					self.skip_spaces();
					if let Ok(cycle_num) = self.parse_number() {
						// TODO: Bounds check?
						actions.push(Action::SetCycle{status_index, cycle: cycle_num as i16});
					}
					self.read_to_end_of_line();
					self.skip_new_line();
				}
				b"die" => {
					// Die does not actually execute if there is a scroll that needs to be shown,
					// which makes sense because if it dies, it would not keep running and therefore
					// never show the scroll. This means if you have text with a link and then #die,
					// die will not execute, and if the link is clicked, it can avoid the #die that
					// is about to be executed.
					// Interestingly, if it's just a one line thing that doesn't open in a scroll,
					// it will die immediately AND show the text (which can be proved by #sending
					// from other objects to this one, and observing that the #sends can't intercept
					// the execution between the text and #die lines.
					// It's possible this happens because the scroll showing function is called
					// in-line in the OOP execution function. I hope that's not the case...
					if state.text_message_content_lines.len() <= 1 {
						// When it's not going to open a scroll, even when it's going to show a one
						// line message at this bottom, it is okay to make a die action immediately
						// because finish_immediately will cause the one line message to be shown
						// first, then delete the tile/status.
						actions.push(Action::SetTile {
							x: status.location_x as i16,
							y: status.location_y as i16,
							tile: BoardTile {
								element_id: ElementType::Empty as u8,
								colour: 0,
							},
							status_element: None,
						});
						outcome.finish_immediately = true;
					} else {
						// When a scroll is going to open, don't execute the die command just yet,
						// because a link in the scroll might jump elsewhere.
						outcome.finish_immediately = false;
						outcome.dont_progress = true;
					}
				}
				b"end" => {
					outcome.finish_immediately = true;
					self.pos = -1;
					//outcome.dont_progress = true;
				}
				b"endgame" => {
					actions.push(Action::ModifyPlayerItem{
						item_type: PlayerItemType::Health,
						offset: -sim.world_header.player_health,
						require_exact_amount: false,
					});
					self.read_to_end_of_line();
					self.skip_new_line();
					// Surprisingly, #endgame actually keeps executing the script, so you can make a
					// scroll appear with working links and stuff after the #endgame invocation.
				}
				b"give" => {
					self.skip_spaces();
					let item_type = self.parse_player_item()?;
					self.skip_spaces();
					// TODO: Check bounds
					if let Ok(give_num) = self.parse_number() {
						self.read_to_end_of_line();
						self.skip_new_line();
						if item_type == PlayerItemType::Time {
							actions.push(Action::ModifyPlayerItem{
								item_type,
								offset: -(give_num as i16),
								require_exact_amount: true,
							});
						} else {
							actions.push(Action::ModifyPlayerItem{
								item_type,
								offset: give_num as i16,
								require_exact_amount: false,
							});
						}
					}
				}
				b"go" => {
					self.skip_spaces();

					let direction = self.parse_direction(status, sim)?;

					// For some reason, `#go i` doesn't actually progress after it idles, so it is
					// effectively `#end`.
					if direction != Direction::Idle {
						let (offset_x, offset_y) = direction.to_offset();
						actions.push(Action::MoveTile{
							from_x: status.location_x as i16,
							from_y: status.location_y as i16,
							to_x: status.location_x as i16 + offset_x,
							to_y: status.location_y as i16 + offset_y,
							offset_x,
							offset_y,
							check_push: true,
							is_player: false,
						});

						self.read_to_end_of_line();
						self.skip_new_line();

						// If it's not blocked after doing MoveOwnTile, then it will jump to the position
						// after the direction that was just parsed.
						state.action_to_check_on_next_step = Some(OopAsyncAction::Move {
							instruction_when_not_blocked: self.pos,
						});
					}

					// Otherwise, when it's blocked, just keep trying again and again.
					outcome.dont_progress = true;
				}
				b"idle" => {
					self.read_to_end_of_line();
					self.skip_new_line();

					outcome.finish_immediately = true;
				}
				b"if" => {
					let if_passed = self.parse_if_predicate(status, sim)?;

					if if_passed {
						outcome = self.parse_command(status_index, status, actions, state, sim)?;
					} else {
						self.read_to_end_of_line();
						self.skip_new_line();
					}
				}
				b"lock" => {
					actions.push(Action::SetStatusParam2{value: 1, status_index});
					self.read_to_end_of_line();
					self.skip_new_line();
				}
				b"play" => {
					let notes = self.read_to_end_of_line();
					self.skip_new_line();
					actions.push(Action::SendBoardMessage(BoardMessage::PlaySoundArray(process_notes_string(&notes.data), SoundPriority::Music)));
				}
				b"put" => {
					self.skip_spaces();
					let direction = self.parse_direction(status, sim)?;
					self.skip_spaces();
					let put_desc = self.parse_tile_type_desc()?;
					self.read_to_end_of_line();
					self.skip_new_line();

					let (off_x, off_y) = direction.to_offset();
					if off_x == 0 && off_y == 0 {
						return Err(DosString::from_slice(b"Bad #PUT"));
					} else {
						let dest_x = status.location_x as i16 + off_x;
						let dest_y = status.location_y as i16 + off_y;

						// For some reason, you can't #put something on the bottom row.
						if dest_y < BOARD_HEIGHT as i16 - 2
							&& dest_y >= 1
							&& dest_x < BOARD_WIDTH as i16 - 1
							&& dest_x >= 1
						{
							actions.push(Action::PushTile{
								x: dest_x,
								y: dest_y,
								offset_x: off_x,
								offset_y: off_y,
							});

							state.action_to_check_on_next_step = Some(OopAsyncAction::Put {
								direction,
								tile_type: put_desc,
							});
						}
					}
				}
				// Shouldn't there be a "restart" command right about here? NO! #restart actaully
				// results in jumping to the local label "restart", so it's actually a
				// `#send restart` invocation.
				b"restore" => {
					// For some reason, when you #restore x, all local x's will be restored, but if
					// you #restore [anything]:x, only one x from each matching object will be
					// restored, even if the current object matches.
					// Upon further inspection, it seems like this is the reason for the above
					// behaviour:
					/*
					# A message string is "object_name:label_name", as used with #send, #zap, #restore

					def find_first_label(string_to_search_for, label_name): ...
					def find_next_label(string_to_search_for, label_name, search_start_pos): ...

					message = parse_message()
					for status in status_elements:
						code = get_status_code(status)
						pos = find_first_label(code, "\r'", label_name)
						while pos >= 0:
							# Change the ' to a :
							code[pos] = ":"
							# Note how on this line, object_name is passed, where label_name should have been:
							pos = find_next_label(code, "\r'", object_name, pos)
					*/
					// This is just a simplification of what actually happens, but it's close
					// enough. So, imagine this scenario:
					// status1: #restore a:b
					// status2: @a\r'b\r'b\r'a\r
					// Then when status1 code runs, it will restore status2's first `b` label, then
					// when it looks for the next one, it will restore the `a` label, because it's
					// now looking for the object name instead of the label.

					self.skip_spaces();
					let send_message_desc = self.parse_message();
					self.read_to_end_of_line();
					self.skip_new_line();

					self.apply_message_desc_label_operation(send_message_desc, LabelOperation::RestoreZztStyle, status_index, actions);
				}
				b"send" => {
					self.skip_spaces();
					let send_message_desc = self.parse_message();
					self.read_to_end_of_line();
					self.skip_new_line();

					self.apply_message_desc_label_operation(send_message_desc, LabelOperation::Jump, status_index, actions);
				}
				b"set" => {
					self.skip_spaces();
					let flag_name = self.read_word();
					self.read_to_end_of_line();
					self.skip_new_line();
					//println!("#set {:?}", flag_name);
					actions.push(Action::SetFlag(flag_name));
				}
				b"shoot" => {
					self.skip_spaces();
					let direction = self.parse_direction(status, sim)?;
					self.read_to_end_of_line();
					self.skip_new_line();

					let (shoot_step_x, shoot_step_y) = direction.to_offset();

					let shoot_x = status.location_x as i16 + shoot_step_x;
					let shoot_y = status.location_y as i16 + shoot_step_y;
					let fired_shot = sim.make_shoot_actions(shoot_x, shoot_y, shoot_step_x, shoot_step_y, false, false, actions);
					if fired_shot {
						actions.push(Action::SendBoardMessage(BoardMessage::PlaySoundArray(
							process_notes_string(b"tc-f#"), SoundPriority::Level(2))));
					}
					outcome.finish_immediately = true;
				}
				b"take" => {
					self.skip_spaces();
					// TODO: Check bounds
					let item_type = self.parse_player_item()?;
					self.skip_spaces();
					if let Ok(take_num) = self.parse_number() {
						// Note that although the manual says a message can come after the number, it is
						// actually a full command that goes there. If the take fails, it will try to
						// run the "message" the same way it runs a command after a # character
						// (it calls parse_command).

						if item_type == PlayerItemType::Time {
							actions.push(Action::ModifyPlayerItem{
								item_type,
								offset: take_num as i16,
								require_exact_amount: false,
							});
						} else {
							actions.push(Action::ModifyPlayerItem{
								item_type,
								offset: -(take_num as i16),
								require_exact_amount: true,
							});
						}

						state.action_to_check_on_next_step = Some(OopAsyncAction::Take);
					}
					// DO NOT read to the end of the line here. The action_to_check_on_next_step
					// will decide whether or not to process the rest of the line as a command, or
					// just skip it.
				}
				b"then" => {
					outcome = self.parse_command(status_index, status, actions, state, sim)?;
				}
				b"throwstar" => {
					self.skip_spaces();
					let direction = self.parse_direction(status, sim)?;
					self.read_to_end_of_line();

					if direction != Direction::Idle {
						let (offset_x, offset_y) = direction.to_offset();
						let dest_x = status.location_x as i16 + offset_x;
						let dest_y = status.location_y as i16 + offset_y;
						// param2 starts at 255 for #put [dir] star, but starts at 99 for #throwstar.
						sim.make_shoot_actions(dest_x, dest_y, offset_x, offset_y, true, false, actions);
					}
				}
				b"try" => {
					self.skip_spaces();

					let direction = self.parse_direction(status, sim)?;
					// DO NOT read to the end of the line here. The action_to_check_on_next_step
					// will decide whether or not to process the rest of the line as a command, or
					// just skip it.

					// `#try i` is a no-op, unlike `?i`, which actually pauses execution.
					if direction != Direction::Idle {
						let (offset_x, offset_y) = direction.to_offset();
						actions.push(Action::MoveTile{
							from_x: status.location_x as i16,
							from_y: status.location_y as i16,
							to_x: status.location_x as i16 + offset_x,
							to_y: status.location_y as i16 + offset_y,
							offset_x,
							offset_y,
							check_push: true,
							is_player: false,
						});

						state.action_to_check_on_next_step = Some(OopAsyncAction::TryMove);
					}
				}
				b"unlock" => {
					actions.push(Action::SetStatusParam2{value: 0, status_index});
					self.read_to_end_of_line();
					self.skip_new_line();
				}
				b"walk" => {
					self.skip_spaces();
					let direction = self.parse_direction(status, sim)?;
					self.read_to_end_of_line();
					self.skip_new_line();

					let (step_x, step_y) = direction.to_offset();
					actions.push(Action::SetStep{status_index, step_x, step_y});
				}
				b"zap" => {
					self.skip_spaces();
					let send_message_desc = self.parse_message();
					self.read_to_end_of_line();
					self.skip_new_line();

					self.apply_message_desc_label_operation(send_message_desc, LabelOperation::Zap, status_index, actions);
				}
				_ => {
					// TODO: Check what happens when the label doesn't exist. Does it skip the new
					// line char?

					// The name after the hash can only be a "myself" message at this point, so
					// directly jumping to label is fine.
					let jump_worked = self.jump_to_label(&command_name);
					if !jump_worked {
						// TODO "Unknown command: {:?}"
						let mut error_string = DosString::from_slice(b"Unknown command: ");
						error_string += &command_name.data;
						return Err(error_string);
					}
				}
			}
		}

		Ok(outcome)
	}

	/// This expects label_to_find to be lower case.
	/// This function is weird, because a destination label will match if the label starts with the
	/// source label, and the next character is not a letter or underscore.
	/// This means that `#send L1` will match `L1`, `L11`, `L11B`, but not `L1B` or `L1_`.
	pub fn find_label(&self, label_to_find: &DosString) -> Option<i16> {
		if label_to_find.data == b"restart" {
			// "restart" is a special label that jumps to the start of the program.
			return Some(0);
		}

		let mut parser = OopParser::new(self.code.as_ref(), 0);

		while parser.pos < parser.code.len() as i16 {
			// Reading to the end of the line first prevents labels on the first line of a program from
			// working, just like in the original ZZT.
			parser.read_to_end_of_line();
			parser.skip_new_line();

			if let OopOperator::Label = parser.parse_operator() {
				let mut current_index = 0;

				while current_index < label_to_find.data.len() && parser.pos < parser.code.len() as i16 {
					let find_char = label_to_find.data[current_index].to_ascii_lowercase();
					let match_char = parser.code[parser.pos as usize + current_index].to_ascii_lowercase();
					if find_char != match_char {
						break;
					} else {
						current_index += 1;
					}
				}

				if current_index == label_to_find.len() {
					let char_after = parser.code[parser.pos as usize + current_index];
					if (char_after >= b'A' && char_after <= b'Z') || (char_after >= b'a' && char_after <= b'z') || char_after == b'_' {
						// Then the label doesn't match.
					} else {
						// Jumping to a label places the cursor on the new line character at the end of
						// the line, skipping anything in between.
						parser.read_to_end_of_line();
						return Some(parser.pos);
					}
				}
			}
		}
		None
	}

	// Returns true if the label was found and jumped to.
	pub fn jump_to_label(&mut self, label: &DosString) -> bool {
		if let Some(label_pos) = self.find_label(label) {
			self.pos = label_pos;
			true
		} else {
			false
		}
	}

	pub fn zap_label(&mut self, label_to_find: &DosString) {
		let mut zap_pos_opt = None;

		{
			let mut parser = OopParser::new(self.code.as_ref(), 0);

			while parser.pos < parser.code.len() as i16 {
				// Reading to the end of the line first prevents labels on the first line of a program from
				// working, just like in the original ZZT.
				parser.read_to_end_of_line();
				parser.skip_new_line();

				let op_pos = parser.pos;

				if let OopOperator::Label = parser.parse_operator() {
					let label = parser.read_word().to_lower();
					if label == *label_to_find {
						zap_pos_opt = Some(op_pos);
						break;
					}
				}
			}
		}

		if let Some(zap_pos) = zap_pos_opt {
			self.code.to_mut().data[zap_pos as usize] = b'\'';
		}
	}

	/// Convert `'label_to_find` in the code to `:label_to_find`. You would think that this only
	/// requires the `label_to_find` to work, but in the original ZZT, if there is an receiver name
	/// specified in a message string (eg. "dude:bombed", where "dude" is the receiver name), all
	/// labels that aren't the very first label will be matched against the receiver name instead of
	/// the label name. I think this is just a mistake, but now it's a feature!
	/// For example, running the following:
	/// status1: `#restore a:b`
	/// status2: `@a\r'b\r'b\r'a\r`
	/// The status2 code will become `@a\r:b\r'b\r:a\r`.
	pub fn restore_labels(&mut self, receiver_name_opt: Option<&DosString>, label_to_find: &DosString) {
		let mut restore_positions = vec![];

		let mut parser = OopParser::new(self.code.as_ref(), 0);
		let mut is_first_match = true;

		while parser.pos < parser.code.len() as i16 {
			// Reading to the end of the line first prevents labels on the first line of a program from
			// working, just like in the original ZZT.
			parser.read_to_end_of_line();
			parser.skip_new_line();

			let op_pos = parser.pos;

			if let OopOperator::Comment = parser.parse_operator() {
				let label = parser.read_word().to_lower();
				let has_match = if is_first_match {
					label == *label_to_find
				} else {
					if let Some(receiver_name) = receiver_name_opt {
						// See function comment for this behaviour.
						label == *receiver_name
					} else {
						label == *label_to_find
					}
				};

				if has_match {
					restore_positions.push(op_pos);

					is_first_match = false;
				}
			}
		}

		for restore_pos in restore_positions {
			self.code.to_mut().data[restore_pos as usize] = b':';
		}
	}

	/// Returns true if this operation modified the parser position.
	pub fn apply_label_operation(&mut self, receiver_name_opt: Option<&DosString>, label: &DosString, label_op: LabelOperation) -> bool {
		match label_op {
			LabelOperation::Jump => { self.jump_to_label(label) }
			LabelOperation::Zap => { self.zap_label(label); false }
			LabelOperation::RestoreZztStyle => { self.restore_labels(receiver_name_opt, label); false }
		}
	}

	/// Gets the first word after the @ sign.
	pub fn get_name(&self) -> Option<DosString> {
		let mut name_parser = OopParser::new(self.code.as_ref(), 0);
		let first_op = name_parser.parse_operator();
		if let OopOperator::Name = first_op {
			Some(name_parser.read_word())
		} else {
			None
		}
	}

	/// This is different from get_name because it reads the entire thing after the @ sign, for use
	/// when displaying scrolls. Normally things only care about the first word, which is what
	/// get_name does.
	pub fn get_scroll_title(&self) -> Option<DosString> {
		let mut name_parser = OopParser::new(self.code.as_ref(), 0);
		let first_op = name_parser.parse_operator();
		if let OopOperator::Name = first_op {
			Some(name_parser.read_to_end_of_line())
		} else {
			None
		}
	}
}
