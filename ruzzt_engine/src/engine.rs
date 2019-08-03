use crate::board_simulator::*;
use crate::scroll::*;
use crate::event::*;
use crate::caption::*;
use crate::console::*;
use crate::behaviour::*;
use crate::board_message::*;
use crate::direction::*;
use crate::oop_parser::OopParser;
use crate::side_bar::{self, SideBar};
use crate::zzt_behaviours;
use crate::sounds::*;

use zzt_file_format::{self, ElementType, BoardTile};
use zzt_file_format::dosstring::DosString;

use num::FromPrimitive;

use std::fs::File;
use std::collections::HashSet;

/// Get the character code associated with the given element type.
/// Note that not all types use this function. For those types it doesn't matter what this returns.
fn element_type_to_char_code(ty: ElementType) -> u8 {
	use self::ElementType::*;
	match ty {
		Empty => 32,
		Player => 2,
		Monitor => 0,
		Torch => 157,
		Solid => 0xdb,
		Breakable => 177,
		Normal => 0xb2,
		Boulder => 254,
		Scroll => 232,
		Door => 0x0a,
		Ammo => 132,
		Head => 0xe9,
		Segment => 0x4f,
		Bear => 0x99,
		Ruffian => 0x05,
		Slime => 0x2a,
		Shark => 0x5e,
		Lion => 0xea,
		Tiger => 0xe3,
		BlinkWall => 0xce,
		SliderNS => 0x12,
		SliderEW => 0x1d,
		Passage => 0xf0,
		Gem => 0x04,
		Ricochet => 0x2a,
		Clockwise => 0x2f,
		Counter => 0x5c,
		Key => 0x0c,
		Invisible => 0x00,
		SpinningGun => 0x0,
		Water => 0xb0,
		Forest => 0xb0,
		Energizer => 0x7f,
		Fake => 0xb2,
		Pusher => 0x0,
		Bomb => 0x0b,
		Duplicator => 0,
		Bullet => 0xf8,
		BlinkRayHorizontal => 0xcd,
		BlinkRayVertical => 0xba,
		Star => 0x0,
		_ => {
			println!("element_type_to_char_code: {:?}", ty);
			0
		},
	}
}

/// Returns true if the given element type is always visible when the room is dark.
fn type_visible_in_dark(ty: ElementType) -> bool {
	match ty {
		ElementType::Player | ElementType::Passage | ElementType::Torch => true,
		_ => false,
	}
}

#[derive(Clone)]
pub struct RuzztEngine {
	/// The `BoardSimulator` used to simulate the current board.
	pub board_simulator: BoardSimulator,
	/// Because a board simulation step can pause halfway through (e.g. to open a scroll), this
	/// stores the state of a partially executed step.
	pub board_simulator_step_state: Option<BoardSimulatorStepState>,
	/// The rendered state of the "console", which stores the characters and colours to display at
	/// each location on the screen, including the sidebar.
	pub console_state: ConsoleState,
	// TODO: Maybe this should just be replaced with things that aren't already stored in
	// BoardSimulator, because right now the board simulator's world_header has to be carefully used
	// all the time, and not the one in this World instance.
	/// The current state of the `World`.
	pub world: zzt_file_format::World,
	/// `global_cycle` is the number of simulation steps since the start of the game.
	pub global_cycle: usize,
	/// Number of times the `RuzztEngine` `step` function has been called since the game was paused.
	/// This is required because the player blinks while the game is paused, but also doesn't
	/// increment the `global_cycle`.
	pub paused_cycle: usize,
	/// If there's a scroll open, this contains the state of the scroll.
	pub scroll_state: Option<ScrollState>,
	/// If there's a caption being displayed, this contains the state of the caption.
	pub caption_state: Option<CaptionState>,
	/// The state of the sidebar on the right of the screen.
	pub side_bar: SideBar,
	/// `OneTimeNotification`s are notifications that are only shown once. When one is shown it is
	/// added to the set so it doesn't get shown again.
	pub shown_one_time_notifications: HashSet<OneTimeNotification>,
	/// When a link in a scroll is pressed, this will be set to that link's target string.
	/// If `board_simulator_step_state` is set, then when the next partial step is executed this
	/// will be used to jump to the associated OOP label on the status currently being processed.
	pub clicked_link_label: Option<DosString>,
	/// True when the game is paused.
	pub is_paused: bool,
	/// True when the game ended and should start simulating really fast. This is not the same as it
	/// being the end of the game, because when the player dies they can use cheat codes to bring
	/// themselves back to life, but the game will continue to simulate fast.
	pub board_should_simulate_fast: bool,
	/// Various result data of actions that have been applied recently.
	/// If the game is paused, then this will just build up and up until the game is unpaused.
	pub accumulated_data: AccumulatedActionData,
	/// True when in the title screen.
	pub in_title_screen: bool,
}

impl RuzztEngine {
	/// Make a new engine with the state of a newly started ZZT game with no world loaded.
	pub fn new() -> RuzztEngine {
		let initial_world = zzt_file_format::World::zzt_default();

		let mut board_simulator = BoardSimulator::new(initial_world.world_header.clone());
		zzt_behaviours::load_zzt_behaviours(&mut board_simulator);

		board_simulator.load_board(&initial_world.boards[initial_world.world_header.player_board as usize]);
		let mut accumulated_data = AccumulatedActionData::new();
		board_simulator.on_player_entered_board(&mut accumulated_data.board_messages);

		let mut engine = RuzztEngine {
			board_simulator,
			board_simulator_step_state: None,
			console_state: ConsoleState::new(),
			world: initial_world,
			global_cycle: 1,
			paused_cycle: 1,
			scroll_state: None,
			caption_state: None,
			side_bar: SideBar::new(),
			shown_one_time_notifications: HashSet::new(),
			clicked_link_label: None,
			is_paused: true,
			board_should_simulate_fast: false,
			accumulated_data,
			in_title_screen: true,
		};

		engine.set_in_title_screen(true);

		engine
	}

	/// Switch between being in-game or in the title screen.
	pub fn set_in_title_screen(&mut self, in_title_screen: bool) {
		self.in_title_screen = in_title_screen;
		if in_title_screen {
			self.board_simulator.load_board(&self.world.boards[0]);
			self.is_paused = false;
		} else {
			self.board_simulator.load_board(&self.world.boards[self.board_simulator.world_header.player_board as usize]);
			self.is_paused = true;
		}
	}

	/// Load the given `world` into the engine to start simulating it. The current `in_title_screen`
	/// value will not change. The board that is loaded initially can be overridden by setting
	/// `start_board` to the desired board's index within the world.
	pub fn load_world(&mut self, mut world: zzt_file_format::World, start_board: Option<i16>) {
		if let Some(start_board) = start_board {
			world.world_header.player_board = start_board;
		}

		let mut board_simulator = BoardSimulator::new(world.world_header.clone());
		zzt_behaviours::load_zzt_behaviours(&mut board_simulator);

		board_simulator.load_board(&world.boards[world.world_header.player_board as usize]);

		let (player_x, player_y) = self.board_simulator.get_player_location();
		self.board_simulator.board_meta_data.player_enter_x = player_x as u8;
		self.board_simulator.board_meta_data.player_enter_y = player_y as u8;

		self.board_simulator = board_simulator;
		self.world = world;
		self.set_in_title_screen(self.in_title_screen);
		self.board_should_simulate_fast = false;
	}

	/// This is true if the game is in "typing" mode, which usually means a text input is open, and
	/// the engine wants `process_typing` to be called instead of `step`.
	pub fn in_typing_mode(&self) -> bool {
		self.side_bar.in_typing_mode()
	}

	// TODO: Don't play sounds when the game is over.
	/// True when the game is over, and all the user can do is press escape to exit to the title
	/// screen.
	pub fn is_end_of_game(&self) -> bool {
		self.board_simulator.world_header.player_health <= 0
	}

	/// See the `board_should_simulate_fast` field in the struct. This doesn't return true if a
	/// scroll or text input is open.
	pub fn should_simulate_fast(&self) -> bool {
		self.board_should_simulate_fast && self.scroll_state.is_none() && !self.side_bar.in_typing_mode()
	}

	/// Returns true if a board simulation step was paused half-way through, such as when a scroll
	/// was opened by an OOP script for example.
	pub fn is_part_way_though_step(&self) -> bool {
		self.board_simulator_step_state.is_some()
	}

	/// Applies the default action for the given `board_message`. For example, it will switch boards
	/// on a `SwitchBoard` or `TeleportToBoard` message. This doens't have any effect for anything
	/// to do with input/output (playing sound, opening worlds from the disk) because those are all
	/// left up to the front-end.
	/// Returns any BoardMessages that happen to be created when `board_message` is applied.
	pub fn process_board_message(&mut self, board_message: BoardMessage) -> Vec<BoardMessage> {
		let mut extra_accumulated_data = AccumulatedActionData::new();

		match board_message {
			BoardMessage::SwitchBoard{new_board_index, direction} => {
				let mut dest_player_pos = self.board_simulator.get_player_location();
				match direction {
					Direction::North => {
						dest_player_pos.1 = BOARD_HEIGHT as i16 - 2;
					}
					Direction::South => {
						dest_player_pos.1 = 1;
					}
					Direction::West => {
						dest_player_pos.0 = BOARD_WIDTH as i16 - 2;
					}
					Direction::East => {
						dest_player_pos.0 = 1;
					}
					_ => {}
				}

				let original_board_index = self.board_simulator.world_header.player_board;
				self.board_simulator.world_header.player_board = new_board_index as i16;

				self.board_simulator.save_board(&mut self.world.boards[original_board_index as usize]);
				self.board_simulator.load_board(&self.world.boards[self.board_simulator.world_header.player_board as usize]);

				let (off_x, off_y) = direction.to_offset();
				// Check if where the player is trying to go on the destination board is blocked.
				let push_blocked = self.board_simulator.push_tile(dest_player_pos.0, dest_player_pos.1, off_x, off_y, true, false, 0, None, &mut extra_accumulated_data);

				if push_blocked == BlockedStatus::NotBlocked {
					let old_board_player_pos = self.board_simulator.get_player_location();
					self.board_simulator.move_tile(old_board_player_pos.0, old_board_player_pos.1, dest_player_pos.0, dest_player_pos.1);
					self.board_simulator.on_player_entered_board(&mut extra_accumulated_data.board_messages);
				} else {
					self.board_simulator.save_board(&mut self.world.boards[self.board_simulator.world_header.player_board as usize]);
					self.board_simulator.world_header.player_board = original_board_index;
					self.board_simulator.load_board(&self.world.boards[self.board_simulator.world_header.player_board as usize]);
				}
			}
			BoardMessage::TeleportToBoard{destination_board_index, passage_colour} => {
				self.board_simulator.save_board(&mut self.world.boards[self.board_simulator.world_header.player_board as usize]);

				self.board_simulator.world_header.player_board = destination_board_index as i16;
				self.board_simulator.load_board(&self.world.boards[self.board_simulator.world_header.player_board as usize]);

				let passage_location_opt = self.board_simulator.get_passage_location(passage_colour);
				if let Some(passage_location) = passage_location_opt {
					let player_location = self.board_simulator.get_player_location();
					//self.board_simulator.move_tile(player_location.0, player_location.1, passage_location.0, passage_location.1);
					// For some reason ZZT manually moves the player when they use a passage, so it
					// can do weird stuff like pick up the tile underneath a player and put it
					// somewhere else.
					self.board_simulator.status_elements[0].location_x = passage_location.0 as u8;
					self.board_simulator.status_elements[0].location_y = passage_location.1 as u8;
					if let Some(old_tile) = self.board_simulator.get_tile_mut(player_location.0, player_location.1) {
						old_tile.element_id = ElementType::Empty as u8;
					}
				}
				self.board_simulator.on_player_entered_board(&mut extra_accumulated_data.board_messages);
				self.is_paused = true;
			}
			BoardMessage::ShowOneTimeNotification(notification_type) => {
				if !self.shown_one_time_notifications.contains(&notification_type) {
					self.caption_state = Some(CaptionState::new(notification_type.message_string()));
					self.shown_one_time_notifications.insert(notification_type);
				}
			}
			BoardMessage::OpenScroll{title, content_lines} => {
				if content_lines.len() > 1 {
					self.scroll_state = Some(ScrollState::new_title_content(title, content_lines));
				} else if content_lines.len() == 1 {
					self.caption_state = Some(CaptionState::new(content_lines[0].clone()));
				}
			}
			BoardMessage::CloseScroll => {
				self.scroll_state = None;
			}
			BoardMessage::PlaySoundArray(..) => {
				// Do nothing. The frontend should handle this itself.
			}
			BoardMessage::ClearPlayingSound => {
				// Do nothing. The frontend should handle this itself.
			}
			BoardMessage::OpenSaveGameInput => {
				self.side_bar.open_text_input(side_bar::TextInputMode::SaveFile, b"SAVED");
			}
			BoardMessage::SaveGameToFile(file_name) => {
				self.sync_world();
				println!("Save to {:?}", file_name);
				if let Ok(mut file) = File::create(file_name.to_string(false)) {
					if let Err(err) = self.world.write(&mut file) {
						println!("Couldn't write to {:?}: {:?}", file_name, err);
					}
				} else {
					println!("Couldn't open {:?}", file_name);
				}
			}
			BoardMessage::OpenDebugInput => {
				self.side_bar.open_text_input(side_bar::TextInputMode::Debug, b"");
			}
			BoardMessage::DebugCommand(command) => {
				match command.to_lower().data.as_slice() {
					b"ammo" => {
						self.board_simulator.world_header.player_ammo += 5;
					}
					b"torches" => {
						if let Some(ref mut player_torches) = self.board_simulator.world_header.player_torches {
							*player_torches += 5;
						}
					}
					b"gems" => {
						self.board_simulator.world_header.player_gems += 5;
					}
					b"health" => {
						self.board_simulator.world_header.player_health += 50;
					}
					b"zap" => {
						let player_pos = self.board_simulator.get_player_location();
						let mut report = ApplyActionResultReport::new();
						let mut zap_at_offset = |off_x, off_y| {
							let action = Action::SetTile{
								x: player_pos.0 + off_x,
								y: player_pos.1 + off_y,
								tile: BoardTile {
									element_id: ElementType::Empty as u8,
									colour: 0,
								},
								status_element: None,
							};
							self.board_simulator.apply_action(player_pos.0 + off_x, player_pos.1 + off_y, action, 0, None, &mut self.accumulated_data, &mut report);
						};
						zap_at_offset(-1, 0);
						zap_at_offset(1, 0);
						zap_at_offset(0, -1);
						zap_at_offset(0, 1);
					}
					b"dark" => {
						self.board_simulator.board_meta_data.is_dark = true;
					}
					b"-dark" => {
						self.board_simulator.board_meta_data.is_dark = false;
					}
					_ => {
						self.caption_state = Some(CaptionState::new(DosString::from_slice(b"Unknown debug command")));
					}
				}
				// TODO: Play a note.
			}
			BoardMessage::LinkClicked(link_label) => {
				// TODO: If link_label starts with "-", then treat it as a file name to load.
				self.clicked_link_label = Some(link_label);
			}
			BoardMessage::PauseGame => {
				if !self.is_end_of_game() {
					self.is_paused = true;
				}
			}
			BoardMessage::PlayGame => {
				self.set_in_title_screen(false);
				extra_accumulated_data.board_messages.push(BoardMessage::ClearPlayingSound);
			}
			BoardMessage::OpenEndGameConfirmation => {
				self.side_bar.open_yes_no_input(side_bar::YesNoMode::EndGame);
			}
			BoardMessage::OpenQuitConfirmation => {
				self.side_bar.open_yes_no_input(side_bar::YesNoMode::Quit);
			}
			BoardMessage::ReturnToTitleScreen => {
				self.set_in_title_screen(true);
			}
			| BoardMessage::Quit
			| BoardMessage::OpenWorldSelection
			| BoardMessage::OpenSaveSelection
			| BoardMessage::OpenWorld{..}
			| BoardMessage::EnterPressedInScroll{..} => {
				// Do nothing. The frontend should handle these itself.
			}
		}

		extra_accumulated_data.board_messages
	}

	/// Open a scroll with the given `title` and `content_lines`.
	pub fn open_scroll(&mut self, title: DosString, content_lines: Vec<DosString>) {
		self.scroll_state = Some(ScrollState::new_title_content(title, content_lines));
	}

	/// Copy the data out of the `BoardSimulator` back into the `World` instance in `RuzztEngine`.
	pub fn sync_world(&mut self) {
		let current_board_index = self.board_simulator.world_header.player_board;
		self.board_simulator.save_board(&mut self.world.boards[current_board_index as usize]);
		self.world.world_header = self.board_simulator.world_header.clone();
	}

	/// Returns true if the given `x`/`y` position on the board is currently not lit (so it's on a
	/// dark board, and is not lit by a torch).
	fn is_position_dark(&self, x: i16, y: i16) -> bool {
		if let Some(torch_cycles) = self.board_simulator.world_header.torch_cycles {
			if torch_cycles > 0 {
				let (player_x, player_y) = self.board_simulator.get_player_location();

				let circle_height = CIRCLE_MASK.len() as i16;
				let top_left_x = player_x - 1 - ((CIRCLE_MASK_WIDTH as i16 - 1) / 2);
				let top_left_y = player_y - 1 - ((circle_height - 1) / 2);

				if x >= top_left_x && x < top_left_x + CIRCLE_MASK_WIDTH as i16
					&& y >= top_left_y && y < top_left_y + circle_height
				{
					let circle_x = x - top_left_x;
					let circle_y = y - top_left_y;
					let ref circle_row = CIRCLE_MASK[circle_y as usize];
					(circle_row >> circle_x & 1) == 0
				} else {
					true
				}
			} else {
				true
			}
		} else {
			false
		}
	}

	/// Get the `ConsoleChar` representing how the given `tile` at the given `tile_x`/`tile_y`
	/// position should look on the screen. This does not account for special drawing that requires
	/// a `StatusElement` to work. In this case, it doesn't matter what it returns, because it
	/// should be overwritten by a later call to `render_status_element_tiles()`.
	fn render_tile(&self, tile: &zzt_file_format::BoardTile, tile_x: usize, tile_y: usize) -> ConsoleChar {
		let char_code;
		let mut background = ConsoleColour::Black;
		let mut foreground = ConsoleColour::Black;

		if let Some(ty) = ElementType::from_u8(tile.element_id) {
			use self::ElementType::*;

			let mut override_colours = false;

			if self.board_simulator.board_meta_data.is_dark {
				if !type_visible_in_dark(ty) && self.is_position_dark(tile_x as i16, tile_y as i16) {
					return ConsoleChar {
						char_code: 0xb0,
						background: ConsoleColour::Black,
						foreground: ConsoleColour::White,
					};
				}
			}

			match ty {
				Empty => {
					char_code = 0;
					background = ConsoleColour::Black;
					foreground = ConsoleColour::Black;
					override_colours = true;
				}
				TextBlue | TextGreen | TextCyan | TextRed | TextPurple | TextBrown | TextBlack => {
					char_code = tile.colour;
					foreground = ConsoleColour::White;

					background = match ty {
						TextBlue => ConsoleColour::Blue,
						TextGreen => ConsoleColour::Green,
						TextCyan => ConsoleColour::Cyan,
						TextRed => ConsoleColour::Red,
						TextPurple => ConsoleColour::Magenta,
						TextBrown => ConsoleColour::Brown,
						TextBlack => ConsoleColour::Black,
						_ => ConsoleColour::Black,
					};
					override_colours = true;
				}
				Object => {
					char_code = 0;
				}
				Transporter => {
					char_code = 0;
				}
				Line => {
					let check_adjacent = |offset_x, offset_y| {
						let off_tile_x = tile_x as i16 + offset_x;
						let off_tile_y = tile_y as i16 + offset_y;

						if off_tile_x < 0 || off_tile_x >= BOARD_WIDTH as i16 || off_tile_y < 0 || off_tile_y >= BOARD_HEIGHT as i16 {
							true
						} else {
							let adjacent_tile = self.board_simulator.get_tile(off_tile_x + 1, off_tile_y + 1).unwrap();
							if let Some(ElementType::Line) | Some(ElementType::BoardEdge) = ElementType::from_u8(adjacent_tile.element_id) {
								true
							} else {
								false
							}
						}
					};

					let join_n = check_adjacent(0, -1);
					let join_s = check_adjacent(0, 1);
					let join_e = check_adjacent(1, 0);
					let join_w = check_adjacent(-1, 0);

					char_code = match (join_n, join_s, join_e, join_w) {
						(false, false, false, false) => 0xfa,
						(false, false, false, true) => 0xb5,
						(false, false, true, false) => 0xc6,
						(false, false, true, true) => 0xcd,
						(false, true, false, false) => 0xd2,
						(false, true, false, true) => 0xbb,
						(false, true, true, false) => 0xc9,
						(false, true, true, true) => 0xcb,
						(true, false, false, false) => 0xd0,
						(true, false, false, true) => 0xbc,
						(true, false, true, false) => 0xc8,
						(true, false, true, true) => 0xca,
						(true, true, false, false) => 0xba,
						(true, true, false, true) => 0xb9,
						(true, true, true, false) => 0xcc,
						(true, true, true, true) => 0xce,
					}
				}
				Player => {
					if self.is_paused {
						char_code = 2;
						override_colours = true;
						background = ConsoleColour::Blue;
						foreground = ConsoleColour::White;
					} else {
						if self.board_simulator.world_header.energy_cycles > 0 {
							char_code = 1;
						} else {
							char_code = 2;
						}
					}
				}
				_ => {
					char_code = element_type_to_char_code(ty);
				}
			}

			if !override_colours {
				background = ConsoleColour::from_u8(tile.colour >> 4).unwrap();
				foreground = ConsoleColour::from_u8(tile.colour & 0b1111).unwrap();
			}
		} else {
			background = ConsoleColour::Black;
			foreground = ConsoleColour::Black;
			char_code = 0;
		}

		ConsoleChar {
			char_code,
			background,
			foreground,
		}
	}

	/// Go through all the `StatusElements` and update their appearance in the console if they
	/// require special rendering. For example, Object elements use their `param1` to determine the
	/// console character to use.
	fn render_status_element_tiles(&mut self) {
		// Note that the game seems to draw the even status elements first, then the odd ones (or
		// maybe the other way around?). This likely doesn't affect the excecution order of objects.

		// The first status is always the player.
		let mut is_first_status = true;

		for status_element in &self.board_simulator.status_elements {
			let x = status_element.location_x as usize;
			let y = status_element.location_y as usize;

			if x < 1 || y < 1 {
				continue;
			}

			let screen_x = x - 1;
			let screen_y = y - 1;

			let tile_opt = self.board_simulator.get_tile(x as i16, y as i16);
			if let Some(tile) = tile_opt {
				if let Some(ty) = ElementType::from_u8(tile.element_id) {
					if self.board_simulator.board_meta_data.is_dark {
						if !type_visible_in_dark(ty) && self.is_position_dark(screen_x as i16, screen_y as i16) {
							// Don't draw any statuses in the darkness.
							continue;
						}
					}

					match ty {
						ElementType::Bomb => {
							if status_element.param1 > 1 {
								self.console_state.get_char_mut(screen_x, screen_y).char_code = b'0' + status_element.param1;
							}
						}
						ElementType::Clockwise => {
							let frame_index = (self.global_cycle % (4 * status_element.cycle) as usize) / status_element.cycle as usize;

							self.console_state.get_char_mut(screen_x, screen_y).char_code = match frame_index {
								0 => 0x2f,
								1 => 0xc4,
								2 => 0x5c,
								3 => 0xb3,
								_ => 0,
							};
						}
						ElementType::Counter => {
							let frame_index = (self.global_cycle % (4 * status_element.cycle) as usize) / status_element.cycle as usize;

							self.console_state.get_char_mut(screen_x, screen_y).char_code = match frame_index {
								0 => 0x5c,
								1 => 0xc4,
								2 => 0x2f,
								3 => 0xb3,
								_ => 0,
							};
						}
						ElementType::Duplicator => {
							self.console_state.get_char_mut(screen_x, screen_y).char_code = match status_element.param1 {
								0 => 0xfa,
								1 => 0xf9,
								2 => 0xf8,
								3 => 0x6f,
								4 => 0x4f,
								// TODO: Check this with ZZT:
								_ => 0,
							};
						}
						ElementType::Object => {
							self.console_state.get_char_mut(screen_x, screen_y).char_code = status_element.param1;
						}
						ElementType::Player => {
							let mut screen_char = self.console_state.get_char_mut(screen_x, screen_y);
							if self.is_paused {
								if is_first_status {
									screen_char.char_code = 0;
									screen_char.background = ConsoleColour::Black;
									screen_char.foreground = ConsoleColour::Black;
								} else {
									screen_char.char_code = 0x02;
									screen_char.background = ConsoleColour::Blue;
									screen_char.foreground = ConsoleColour::White;
								}
							}
						}
						ElementType::Pusher => {
							let facing_dir = match (status_element.step_x, status_element.step_y) {
								(1, 0) => Direction::East,
								(-1, 0) => Direction::West,
								(0, -1) => Direction::North,
								(0, 1) => Direction::South,
								_ => Direction::South,
							};

							self.console_state.get_char_mut(screen_x, screen_y).char_code = match facing_dir {
								Direction::East => 0x10,
								Direction::West => 0x11,
								Direction::North => 0x1e,
								Direction::South => 0x1f,
								Direction::Idle => 0x1f,
							};
						}
						ElementType::SpinningGun => {
							let frame_index = (self.global_cycle % (4 * status_element.cycle) as usize) / status_element.cycle as usize;

							self.console_state.get_char_mut(screen_x, screen_y).char_code = match frame_index {
								0 => 0x18,
								1 => 0x1a,
								2 => 0x19,
								3 => 0x1b,
								_ => 0,
							};
						}
						ElementType::Star => {
							//let frame_offset = (self.global_cycle + (status_element.param2 as usize)) % 2;
							//let frame_index = ((self.global_cycle & !1) + frame_offset) % 4;
							let frame_index = (self.global_cycle % (4 * status_element.cycle) as usize) / status_element.cycle as usize;
							self.console_state.get_char_mut(screen_x, screen_y).char_code = match frame_index {
								0 => 0x2f,
								1 => 0xc4,
								2 => 0x5c,
								3 => 0xb3,
								_ => 0,
							};
						}
						ElementType::Transporter => {
							// ZZT shows weird animations for this when the step_x/y is > 1.
							let facing_dir = match (status_element.step_x, status_element.step_y) {
								(1, 0) => Direction::East,
								(-1, 0) => Direction::West,
								(0, -1) => Direction::North,
								(0, 1) => Direction::South,
								_ => Direction::East,
							};
							let mut frame_index = (self.global_cycle % (4 * status_element.cycle) as usize) / status_element.cycle as usize;
							if frame_index == 3 {
								frame_index = 1;
							}

							self.console_state.get_char_mut(screen_x, screen_y).char_code = match (facing_dir, frame_index) {
								(Direction::East, 0) => 0x3e,
								(Direction::East, 1) => 0x29,
								(Direction::East, 2) => 0xb3,
								(Direction::West, 0) => 0x3c,
								(Direction::West, 1) => 0x28,
								(Direction::West, 2) => 0xb3,
								(Direction::North, 0) => 0x7e,
								(Direction::North, 1) => 0x5e,
								(Direction::North, 2) => 0xc4,
								(Direction::South, 0) => 0x5f,
								(Direction::South, 1) => 0x76,
								(Direction::South, 2) => 0xc4,
								_ => 0,
							};
						}
						_ => {}
					}
				}
			}

			if is_first_status && self.is_paused && self.paused_cycle % 10 < 5 {
				let mut screen_char = self.console_state.get_char_mut(screen_x, screen_y);
				screen_char.char_code = 0x02;
				screen_char.background = ConsoleColour::Blue;
				screen_char.foreground = ConsoleColour::White;
			}

			is_first_status = false;
		}
	}

	/// Update the entire console state by drawing the board, side bar, scroll, caption, etc.
	pub fn update_screen(&mut self) {
		// TODO: The game gives the appearance of health being the value when #endgame was invoked
		// because it doesn't redraw the side bar while the game is over.
		self.side_bar.draw_side_bar(&self.board_simulator.world_header, &self.board_simulator.board_meta_data, self.is_paused, self.in_title_screen, &mut self.console_state, self.paused_cycle);

		for y in 0 .. BOARD_HEIGHT - 2 {
			for x in 0 .. BOARD_WIDTH - 2 {
				let ref tile = self.board_simulator.get_tile(x as i16 + 1, y as i16 + 1).unwrap();

				*self.console_state.get_char_mut(x, y) = self.render_tile(tile, x, y);
			}
		}

		self.render_status_element_tiles();

		if let Some(ref caption_state) = self.caption_state {
			caption_state.draw_caption(&mut self.console_state);
		}

		if let Some(ref scroll_state) = self.scroll_state {
			scroll_state.draw_scroll(&mut self.console_state);
		}
	}

	/// When `in_typing_mode()` returns true, this should be called instead of `step`.
	/// This will add characters to text inputs.
	/// Note that `event` is not the same as the `event` passed to `step`.
	pub fn process_typing(&mut self, event: TypingEvent) -> Vec<BoardMessage> {
		self.paused_cycle += 1;
		let board_messages = self.side_bar.process_typing(event, &self.board_simulator.world_header);
		self.update_screen();
		board_messages
	}

	/// Simulate a single game step. A RUZZT front-end will call this over and over, redrawing the
	/// screen between each call. The latest controller input should be passed as `event`.
	/// `global_time_passed_seconds` is the wall-clock time passed since the game started,
	/// regardless of how fast the game is stepping.
	pub fn step(&mut self, event: Event, global_time_passed_seconds: f64) -> Vec<BoardMessage> {
		let was_end_of_game = self.is_end_of_game();

		let mut board_messages = std::mem::replace(&mut self.accumulated_data.board_messages, vec![]);

		if self.is_paused {
			let move_dir = match event {
				Event::Left => Direction::West,
				Event::Right => Direction::East,
				Event::Up => Direction::North,
				Event::Down => Direction::South,
				_ => Direction::Idle,
			};

			if move_dir != Direction::Idle {
				let (off_x, off_y) = move_dir.to_offset();
				let player_status = &self.board_simulator.status_elements[0];
				let player_x = player_status.location_x as i16;
				let player_y = player_status.location_y as i16;
				let blocked = self.board_simulator.push_tile(player_x + off_x, player_y + off_y, off_x, off_y, true, false, 0, None, &mut self.accumulated_data);

				if blocked == BlockedStatus::NotBlocked {
					let player_status = &mut self.board_simulator.status_elements[0];
					let under_element_id = player_status.under_element_id;
					let under_colour = player_status.under_colour;
					player_status.location_x = (player_x + off_x) as u8;
					player_status.location_y = (player_y + off_y) as u8;
					if let Some(old_tile) = self.board_simulator.get_tile_mut(player_x, player_y) {
						if old_tile.element_id == ElementType::Player as u8 {
							old_tile.element_id = under_element_id;
							old_tile.colour = under_colour;
						}
					}
					self.is_paused = false;
				}
			}

			self.paused_cycle += 1;
		} else {
			let mut caption_is_finished = false;
			if let Some(ref mut caption_state) = self.caption_state {
				caption_state.time_left -= 1;
				if caption_state.time_left == 0 {
					caption_is_finished = true;
				}
			}

			if caption_is_finished {
				self.caption_state = None;
			}

			if let Some(ref mut scroll_state) = self.scroll_state {
				board_messages.extend(scroll_state.step(event));
			} else {
				// Force the player status to point at a player tile.
				let (player_x, player_y) = self.board_simulator.get_player_location();
				if self.in_title_screen {
					self.board_simulator.set_tile(player_x, player_y, BoardTile {
						element_id: ElementType::Monitor as u8,
						colour: 0,
					});
				} else {
					self.board_simulator.set_tile(player_x, player_y, BoardTile {
						element_id: ElementType::Player as u8,
						colour: 31,
					});
				}

				let current_global_cycle = self.global_cycle;
				let board_simulator_step_state = self.board_simulator_step_state.get_or_insert_with(|| BoardSimulatorStepState::new(event, current_global_cycle));

				let mut process_same_status = false;

				if let Some(ref clicked_link_label) = self.clicked_link_label {
					if let Some(processing_status_index) = board_simulator_step_state.processing_status_index_opt {
						let current_status = &self.board_simulator.status_elements[processing_status_index];
						let mut parser = OopParser::new(self.board_simulator.get_status_code(current_status), 0);
						parser.jump_to_label(&clicked_link_label);

						let new_code_current_instruction = parser.pos;
						let current_status = &mut self.board_simulator.status_elements[processing_status_index];
						current_status.code_current_instruction = new_code_current_instruction;
					}
					process_same_status = true;
				}

				self.clicked_link_label = None;

				let mut is_done = false;
				// The step pauses as soon as a board message is sent.
				while !is_done && board_simulator_step_state.accumulated_data.board_messages.is_empty() {
					is_done = board_simulator_step_state.partial_step(process_same_status, &mut self.board_simulator);
					process_same_status = false;

					if board_simulator_step_state.accumulated_data.should_check_time_elapsed {
						board_simulator_step_state.accumulated_data.should_check_time_elapsed = false;

						let new_time_passed_ticks = (global_time_passed_seconds * 100.) as i16 % 6000;
						let mut diff = new_time_passed_ticks - self.board_simulator.world_header.time_passed_ticks;
						if diff < 0 {
							diff += 6000;
						}

						if diff >= 100 {
							// At least one second has passed.
							self.board_simulator.world_header.time_passed += 1;
							self.board_simulator.world_header.time_passed_ticks = new_time_passed_ticks;

							if self.board_simulator.board_meta_data.time_limit > 0 {
								let time_left = self.board_simulator.board_meta_data.time_limit - self.board_simulator.world_header.time_passed;

								if time_left == 10 {
									board_messages.push(BoardMessage::OpenScroll{title: DosString::new(), content_lines: vec![DosString::from_slice(b"Running out of time!")]});
								}

								if time_left < 0 {
									self.board_simulator.world_header.player_health = (self.board_simulator.world_header.player_health - 10).max(0);
									self.board_simulator.restart_player_on_board(&mut board_messages);
								}
							}
						}
					}
				}

				board_messages.extend(std::mem::replace(&mut board_simulator_step_state.accumulated_data.board_messages, vec![]));

				if is_done {
					self.board_simulator_step_state = None;
					// Only increment if the whole step is complete, not when it pauses half way through
					// to open a scroll for example.
					self.global_cycle += 1;
				}
			}
		}

		//self.update_screen();

		//println!("{} - {}", self.board_simulator.world_header.player_board, self.world.boards[self.board_simulator.world_header.player_board as usize].meta_data.board_name.to_string(true));

		if self.is_end_of_game() {
			if !was_end_of_game {
				board_messages.push(BoardMessage::PlaySoundArray(process_notes_string(b"s.-cd#g+c-ga#+dgfg#+cf---hc"), SoundPriority::Level(5)));
			}

			if self.global_cycle % 7 == 0 {
				self.caption_state = Some(CaptionState::new(DosString::from_slice(b" Game over  \xc4  Press ESCAPE")));
			}

			self.board_should_simulate_fast = true;
		}

		board_messages
	}
}
