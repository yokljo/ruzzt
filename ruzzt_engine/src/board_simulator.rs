use zzt_file_format::*;
use zzt_file_format::dosstring::DosString;
use crate::event::*;
use crate::direction::*;
use crate::behaviour::*;
use crate::oop_parser::*;
use crate::board_message::*;

use std::borrow::Cow;
use std::cmp::Ordering;
use std::rc::Rc;
use rand::Rng;

// http://apocalyptech.com/games/zzt/manual/langref.html
// http://www.chriskallen.com/zzt/behaviors.html
// http://www.chriskallen.com/zzt/zztoop.php (see LEGACYTICK)
// http://www.chriskallen.com/zzt/zztoop.php#soundfx
// https://museumofzzt.com/file/m/Mwencv14.zip?file=zztop.txt

// These are two larger than 60x25 becuase a border of BoardEdge tiles is added.
pub const BOARD_WIDTH: usize = 62;
pub const BOARD_HEIGHT: usize = 27;

/// This mask is used for torches and bomb explosions.
pub const CIRCLE_MASK: [u16; 9] = [
	0b000111111111000,
	0b001111111111100,
	0b011111111111110,
	0b011111111111110,
	0b111111111111111,
	0b011111111111110,
	0b011111111111110,
	0b001111111111100,
	0b000111111111000,
];

pub const CIRCLE_MASK_WIDTH: usize = 15;

const DEFAULT_BEHAVIOUR: DefaultBehaviour = DefaultBehaviour;

/// The BoardSimulator simulates a single board in a ZZT game world.
/// This simulation is independent of the World state, so before you switch boards, you must replace
/// the respective board data in the World state with the current simulated board state.
///
/// Note that the BoardSimulator is unable to use the `Board` struct directly, because a ZZT board
/// is 60x25 tiles, but the simulation space is 62x27 tiles. This is because ZZT adds an additional
/// border of `BoardEdge` tiles around the edges. This is also why `StatusElement`s seem like they
/// use 1-indexed positions on the `Board`: They are actually 0-indexed once the `BoardEdge`s are
/// added in.
///
/// The board edge is initialised once, which is why if you zap a board edge, then switch boards,
/// the deleted board edge will persist.
#[derive(Debug, Clone)]
pub struct BoardSimulator {
	/// The `WorldHeader` from the `World` (the one that contains the board the `BoardSimulator` is
	/// simulating).
	pub world_header: WorldHeader,
	/// The `BoardMetaData` for the respective `Board` from the `World`.
	pub board_meta_data: BoardMetaData,
	/// The `StatusElement`s for the respective `Board` from the `World`.
	pub status_elements: Vec<StatusElement>,
	/// The tiles. This will be a list of 62*27 tiles, stored in row-first order (so, 0x0, 1x0...).
	/// See struct comment for why it is 62x27 and not 60x25.
	pub tiles: Vec<BoardTile>,
	/// The behaviours associated with `BoardTile` `element_id`s. To find the behaviour for a
	/// particular `ElementType`, cast the ElementType to a u8, then use that to index this list.
	/// The behaviours are loaded into this list via the `set_behaviour` method.
	/// These are `Rc` so that `Behaviour` doesn't need to impl `Clone`.
	pub behaviours: Vec<Option<Rc<Behaviour>>>,
}

impl BoardSimulator {
	pub fn new(world_header: WorldHeader) -> BoardSimulator {
		let mut tiles = vec![];
		for y in 0 .. BOARD_HEIGHT {
			for x in 0 .. BOARD_WIDTH {
				if x == 0 || x == BOARD_WIDTH - 1 || y == 0 || y == BOARD_HEIGHT - 1 {
					tiles.push(BoardTile {
						element_id: ElementType::BoardEdge as u8,
						colour: 0,
					});
				} else {
					tiles.push(BoardTile {
						element_id: ElementType::Empty as u8,
						colour: 0,
					});
				}
			}
		}

		BoardSimulator {
			world_header,
			board_meta_data: BoardMetaData::default(),
			status_elements: vec![],
			tiles,
			behaviours: vec![],
		}
	}

	/// Assign a `Behaviour` to an `ElementType`. This defines how tiles of this type are simulated.
	pub fn set_behaviour(&mut self, element_type: ElementType, behaviour: Box<Behaviour>) {
		let index = element_type as usize;
		while self.behaviours.len() <= index {
			self.behaviours.push(None);
		}
		self.behaviours[index] = Some(behaviour.into());
	}

	/// Get a random unit vector along a direction (N, S, E, W).
	pub fn get_random_step(&self) -> (i16, i16) {
		let mut rng = rand::thread_rng();
		let step_x = rng.gen_range(0, 3) - 1;
		let step_y = if step_x == 0 {
			if rng.gen_range(0, 2) == 0 { -1 } else { 1 }
		} else {
			0
		};
		(step_x, step_y)
	}

	/// Calls `visit_fn` with every tile on the board.
	/// `visit_fn` takes the x/y position of each tile, and the tile itself.
	pub fn visit_all_tiles(&self, visit_fn: &mut FnMut(i16, i16, BoardTile)) {
		for x in (0 .. BOARD_WIDTH).rev() {
			for y in (0 .. BOARD_HEIGHT).rev() {
				visit_fn(x as i16, y as i16, self.get_tile(x as i16, y as i16).unwrap());
			}
		}
	}

	/// Sets the tile at the given x/y position on the board to `tile`.
	/// Returns false if the given position was out of bounds.
	pub fn set_tile(&mut self, x: i16, y: i16, tile: BoardTile) -> bool {
		let index = x + (y * BOARD_WIDTH as i16);
		if index >= 0 && index < self.tiles.len() as i16 {
			self.tiles[index as usize] = tile;
			true
		} else {
			false
		}
	}

	/// Get the tile at the given x/y position, or None if the given position is out of bounds.
	pub fn get_tile(&self, x: i16, y: i16) -> Option<BoardTile> {
		let index = x + (y * BOARD_WIDTH as i16);
		if index >= 0 && index < self.tiles.len() as i16 {
			Some(self.tiles[index as usize])
		} else {
			None
		}
	}

	/// Get a muteable reference to the tile at the given x/y position, or None if the given
	/// position is out of bounds.
	pub fn get_tile_mut(&mut self, x: i16, y: i16) -> Option<&mut BoardTile> {
		let index = x + (y * BOARD_WIDTH as i16);
		if index >= 0 && index < self.tiles.len() as i16 {
			self.tiles.get_mut(index as usize)
		} else {
			None
		}
	}

	/// Get the tile at the location of `status', or None if the location is out of bounds.
	pub fn get_status_tile(&self, status: &StatusElement) -> Option<BoardTile> {
		self.get_tile(status.location_x as i16, status.location_y as i16)
	}

	/// Get the first status in the `status_elements` list with a position matching the input x/y
	/// position, or None if there is no status at that position.
	/// Returns a tuple of (status index, status element).
	pub fn get_first_status_for_pos(&self, x: i16, y: i16) -> Option<(usize, &StatusElement)> {
		for (i, status_element) in self.status_elements.iter().enumerate() {
			if status_element.location_x == x as u8 && status_element.location_y == y as u8 {
				return Some((i, status_element));
			}
		}
		None
	}

	/// Get the first status in the `status_elements` list with a position matching the input x/y
	/// position, or None if there is no status at that position.
	/// Returns a tuple of (status index, status element).
	pub fn get_first_status_for_pos_mut(&mut self, x: i16, y: i16) -> Option<(usize, &mut StatusElement)> {
		for (i, status_element) in self.status_elements.iter_mut().enumerate() {
			if status_element.location_x == x as u8 && status_element.location_y == y as u8 {
				return Some((i, status_element));
			}
		}
		None
	}

	/// Removes statuses for the given x/y position, and updates references within other statuses
	/// as necessary.
	/// Returns a list of removed status indices.
	fn remove_status_for_pos(&mut self, remove_x: i16, remove_y: i16) -> Vec<usize> {
		let mut removed_status_indices = vec![];
		let mut check_removal_index = 0;

		while check_removal_index < self.status_elements.len() {
			let ref elem = self.status_elements[check_removal_index];
			let (x, y) = (elem.location_x, elem.location_y);

			if (x as i16, y as i16) == (remove_x, remove_y) {
				// Step 1: If a status is removed that has another status bound to its code_source,
				//   and it owns code, move the code from the removed one into the first one that
				//   references it, then change any other references to the removed status so they
				//   point to the new one.

				let removing_code_source = std::mem::replace(&mut self.status_elements[check_removal_index].code_source, CodeSource::Owned(DosString::new()));

				if let CodeSource::Owned(mut removing_code) = removing_code_source {
					let mut new_bound_index_opt = None;

					for (index, status) in self.status_elements.iter_mut().enumerate() {
						if status.code_source == CodeSource::Bound(check_removal_index) {
							if let Some(new_bound_index) = new_bound_index_opt {
								status.code_source = CodeSource::Bound(new_bound_index);
							} else {
								status.code_source = CodeSource::Owned(std::mem::replace(&mut removing_code, DosString::new()));
								new_bound_index_opt = Some(index);
							}
						}
					}
				}

				// Step 2: Remove the status, and update any references that point to indices higher
				//   than the removed index.
				self.status_elements.remove(check_removal_index);

				for status in &mut self.status_elements {
					if status.follower >= 0 {
						match check_removal_index.cmp(&(status.follower as usize)) {
							Ordering::Less => {
								status.follower -= 1;
							}
							Ordering::Equal => {
								status.follower = -1;
							}
							_ => {}
						}
					}

					if status.leader >= 0 {
						match check_removal_index.cmp(&(status.leader as usize)) {
							Ordering::Less => {
								status.leader -= 1;
							}
							Ordering::Equal => {
								status.leader = -1;
							}
							_ => {}
						}
					}

					if let CodeSource::Bound(ref mut bound_index) = status.code_source {
						if *bound_index > check_removal_index {
							*bound_index -= 1;
						}
					}
				}

				removed_status_indices.push(check_removal_index);
			} else {
				check_removal_index += 1;
			}
		}

		removed_status_indices
	}

	/// Get the behaviour associated with the given `element_id`.
	fn behaviour_for_element_id(&self, element_id: u8) -> &Behaviour {
		let behaviour_opt = self.behaviours.get(element_id as usize);

		if let Some(Some(behaviour)) = behaviour_opt {
			behaviour.as_ref()
		} else {
			&DEFAULT_BEHAVIOUR
		}
	}

	/// Get the behaviour associated with the given x/y position.
	pub fn behaviour_for_pos(&self, x: i16, y: i16) -> &Behaviour {
		if let Some(tile) = self.get_tile(x, y) {
			self.behaviour_for_element_id(tile.element_id)
		} else {
			&DEFAULT_BEHAVIOUR
		}
	}

	/// Tries to move the tile at `from_x`/`from_y` to `to_x`/`to_y`.
	pub fn move_tile(&mut self, from_x: i16, from_y: i16, to_x: i16, to_y: i16) {
		if from_x == to_x && from_y == to_y {
			return;
		}

		let from_tile = if let Some(tile) = self.get_tile(from_x, from_y) {
			tile
		} else {
			return;
		};

		let to_tile = if let Some(tile) = self.get_tile(to_x, to_y) {
			tile
		} else {
			return;
		};

		self.set_tile(to_x, to_y, from_tile);

		let mut under_element_id = 0;
		let mut under_colour = 0;

		for status_element in &mut self.status_elements {
			if status_element.location_x == from_x as u8 && status_element.location_y == from_y as u8 {
				status_element.location_x = to_x as u8;
				status_element.location_y = to_y as u8;

				under_element_id = status_element.under_element_id;
				under_colour = status_element.under_colour;

				status_element.under_element_id = to_tile.element_id;
				status_element.under_colour = to_tile.colour;
			}
		}

		self.set_tile(from_x, from_y, BoardTile {
			element_id: under_element_id,
			colour: under_colour,
		});
	}

	/// The `push_tile` function is called when one tile tries to move onto another tile, to move
	/// the tile at the destination location out of the way.
	/// Call `push_tile` with the x/y location of the tile to push. `push_off_x` and `push_off_y`
	/// represent the offset from the current position to try to push the tile by.
	/// If the player is pushing the tile, `is_player` should be true.
	/// If the tile is something like a boulder that is able to squash squashable tiles like gems,
	/// then `can_squash' should be true.
	/// `global_cycle` is the number of simulation steps since the start of the game.
	/// `processing_status_index` is the (optional) status index of the tile doing the pushing.
	/// `accumulated_data`: see `AccumulatedActionData`.
	/// Returns a `BlockedStatus` indicating whether or not the push succeeded.
	pub fn push_tile(&mut self,
			x: i16,
			y: i16,
			push_off_x: i16,
			push_off_y: i16,
			is_player: bool,
			can_squash: bool,
			global_cycle: usize,
			processing_status_index: Option<usize>,
			accumulated_data: &mut AccumulatedActionData) -> BlockedStatus {
		if push_off_x == 0 && push_off_y == 0 {
			return BlockedStatus::NotBlocked;
		}

		// Here's the setup: You have a blinkwall facing north, and just to the right of the
		// blinking wall is a pusher with a wall just above it. You walk into the blinkwall
		// just left of the wall tile, and the blinkwall pushes the player on top of the
		// wall. Then, the player is pushed up, and the wall reappears, so the pusher
		// doesn't move anywhere even though it just pushed the player.
		// This is because the way the game checks if a push actually succeeded is if the space is
		// now empty, ie. behaviour.blocked() returns NotBlocked. Only fakes and empties do that.

		let behaviour = self.behaviour_for_pos(x, y);
		let result = behaviour.push(x, y, push_off_x, push_off_y, is_player, self);

		self.apply_action_result(x, y, result.action_result, global_cycle, processing_status_index, accumulated_data);

		// The behaviour can decide to block the way even if it moves (like a scroll does when it
		// dies but the player doesn't move on top of it). But if it says the way isn't block, the
		// game checks to see if theres a freww space before claiming it's not blocked.
		if result.blocked == BlockedStatus::Blocked {
			BlockedStatus::Blocked
		} else {
			let behaviour_after_push = self.behaviour_for_pos(x, y);
			if can_squash && behaviour_after_push.can_be_squashed() {
				BlockedStatus::NotBlocked
			} else {
				behaviour_after_push.blocked(is_player)
			}
		}
	}

	/// Get the location of the player.
	/// Note: The player is ALWAYS status element 0 in ZZT.
	pub fn get_player_location(&self) -> (i16, i16) {
		let ref player_status = self.status_elements[0];
		(player_status.location_x as i16, player_status.location_y as i16)
	}

	/// Check if there is a player tile at the given x/y location.
	/// Note: There can be multiple player tiles on a single board.
	pub fn has_player_at_location(&self, x: i16, y: i16) -> bool {
		if let Some(tile) = self.get_tile(x, y) {
			tile.element_id == ElementType::Player as u8
		} else {
			false
		}
	}

	/// This should be called when the player runs out of time or is hurt on a board that has
	/// `restart_on_zap`. This function moves the player to the board entered location, resets
	/// the board time left, and attempts to pause the game.
	/// `board_messages` is the current list of accumulated board messages.
	pub fn restart_player_on_board(&mut self, board_messages: &mut Vec<BoardMessage>) {
		let (player_x, player_y) = self.get_player_location();
		self.move_tile(player_x, player_y, self.board_meta_data.player_enter_x as i16, self.board_meta_data.player_enter_y as i16);
		board_messages.push(BoardMessage::PauseGame);
		self.world_header.time_passed = 0;
	}

	/// Finds the location of the passage associated with the given colour.
	/// The search starts at the bottom right and goes up, and the function returns the first one it
	/// finds.
	pub fn get_passage_location(&self, colour: u8) -> Option<(i16, i16)> {
		for x in (0 .. BOARD_WIDTH).rev() {
			for y in (0 .. BOARD_HEIGHT).rev() {
				let tile = self.tiles[x + (y * BOARD_WIDTH)];
				if tile.element_id == ElementType::Passage as u8 {
					if tile.colour == colour {
						return Some((x as i16, y as i16));
					}
				}
			}
		}

		None
	}

	/// Attempt to fire a bullet (or a star) from `shoot_start_x`/`shoot_start_y` moving along
	/// `shoot_step_x`/`shoot_step_y`. Set `shoot_star` to true to fire a star instead of a bullet.
	/// Set `shot_by_player` to true if the player is firing.
	/// `actions` is the list of actions to apply. The actions generated by this function will be
	/// appended to this list.
	/// Note that, for example, if the player is shooting a `Breakable` tile that is immediately
	/// adjacent, the tile will be deleted without spawning a bullet.
	/// Returns true if a shot was fired.
	pub fn make_shoot_actions(&self,
			shoot_start_x: i16,
			shoot_start_y: i16,
			shoot_step_x: i16,
			shoot_step_y: i16,
			shoot_star: bool,
			shot_by_player: bool,
			actions: &mut Vec<Action>) -> bool {
		let dest_behaviour = self.behaviour_for_pos(shoot_start_x, shoot_start_y);
		// This is the tile that the bullet is placed on top of, if it is able to be fired at all.
		// It can't be fired if the tile is blocked_for_bullets, or happens to be off the screen.
		let under_tile_opt = self.get_tile(shoot_start_x, shoot_start_y);

		let mut fired_shot = false;

		let mut shooting_allowed = true;

		if shot_by_player {
			if self.board_meta_data.max_player_shots == 0 {
				actions.push(Action::SendBoardMessage(BoardMessage::ShowOneTimeNotification(OneTimeNotification::ShootingNotAllowed)));
				shooting_allowed = false;
			} else {
				let mut existing_player_bullet_count = 0;
				// Count number of player bullets on the screen.
				for status_element in &self.status_elements {
					if let Some(tile) = self.get_status_tile(status_element) {
						if tile.element_id == ElementType::Bullet as u8 && status_element.param1 == 0 {
							existing_player_bullet_count += 1;
						}
					}
				}

				if existing_player_bullet_count >= self.board_meta_data.max_player_shots {
					shooting_allowed = false;
				}
			}
		}

		if shooting_allowed {
			if dest_behaviour.blocked_for_bullets() == BlockedStatus::Blocked {
				if dest_behaviour.destructable() {
					dest_behaviour.damage(shoot_start_x, shoot_start_y, DamageType::Shot{by_player: shot_by_player}, self, actions);
					fired_shot = true;
				}
			} else {
				if let Some(under_tile) = under_tile_opt {
					let (tile, param2) = if shoot_star {
						// Proof that this should use 100 for param2 is that when a tiger shoots a
						// bullet, param2 is set to 100, as if it were shooting a star.
						(BoardTile {
							element_id: ElementType::Star as u8,
							colour: 0xa,
						},
						100)
					} else {
						(BoardTile {
							element_id: ElementType::Bullet as u8,
							colour: 0xf,
						},
						0)
					};

					let bullet_status = StatusElement {
						location_x: shoot_start_x as u8,
						location_y: shoot_start_y as u8,
						step_x: shoot_step_x,
						step_y: shoot_step_y,
						cycle: 1,
						// This is why stars have param1 == 1 when doing #throwstar.
						param1: if shot_by_player { 0 } else { 1 },
						param2,
						under_element_id: under_tile.element_id,
						under_colour: under_tile.colour,
						.. StatusElement::default()
					};

					// Note that this adds the bullet to the end of the status element list, so
					// before this frame is over it will have already moved the bullet by one
					// step. This makes it look like the bullet is never in the adjacent
					// position, which is exactly what happens in the original game.
					actions.push(Action::SetTile {
						x: shoot_start_x,
						y: shoot_start_y,
						tile,
						status_element: Some(bullet_status),
					});
					fired_shot = true
				}
			}
		}

		fired_shot
	}

	/// Initialises the simulated board with the state of a board from the World.
	pub fn load_board(&mut self, board: &Board) {
		self.board_meta_data = board.meta_data.clone();
		self.status_elements = board.status_elements.clone();

		for x in 0 .. BOARD_WIDTH - 2 {
			for y in 0 .. BOARD_HEIGHT - 2 {
				self.set_tile(x as i16 + 1, y as i16 + 1, board.tiles[(x + y*(BOARD_WIDTH - 2)) as usize]);
			}
		}
	}

	/// Updates the state of the given `Board` with the current state of the simulated board.
	pub fn save_board(&self, board: &mut Board) {
		board.meta_data = self.board_meta_data.clone();
		board.status_elements = self.status_elements.clone();

		for x in 0 .. BOARD_WIDTH - 2 {
			for y in 0 .. BOARD_HEIGHT - 2 {
				board.tiles[(x + y*(BOARD_WIDTH - 2)) as usize] = self.get_tile(x as i16 + 1, y as i16 + 1).unwrap();
			}
		}
	}

	/// This is the set_current_location_as_enter_location_and_reset_time_and_show_dark_room_notification function.
	pub fn on_player_entered_board(&mut self, board_messages: &mut Vec<BoardMessage>) {
		let (player_x, player_y) = self.get_player_location();
		self.board_meta_data.player_enter_x = player_x as u8;
		self.board_meta_data.player_enter_y = player_y as u8;
		self.world_header.time_passed = 0;

		if self.board_meta_data.is_dark {
			board_messages.push(BoardMessage::ShowOneTimeNotification(OneTimeNotification::RoomIsDark));
		}
	}

	/// Get the code associated with the status at the given `status_index`.
	/// If the code of the given status is bound to the code of another status, return that code.
	pub fn get_status_index_code(&self, status_index: usize) -> &DosString {
		let mut current_index = status_index;
		loop {
			match self.status_elements[current_index].code_source {
				CodeSource::Owned(ref code) => { return code; }
				CodeSource::Bound(index) => { current_index = index; }
			}
		}
	}

	/// Get the code associated with the status at the given `status_index`.
	/// If the code of the given status is bound to the code of another status, return that code.
	pub fn get_status_index_code_mut(&mut self, status_index: usize) -> &mut DosString {
		// TODO: NLL
		let mut current_index = status_index;
		loop {
			match &mut self.status_elements[current_index].code_source {
				CodeSource::Owned(_) => { break; }
				CodeSource::Bound(index) => { current_index = *index; }
			}
		}

		match self.status_elements[current_index].code_source {
			CodeSource::Owned(ref mut code) => code,
			_ => unreachable!("Loop above makes this impossible"),
		}
	}

	/// Get the code associated with the given status.
	/// If the code of the given status is bound to the code of another status, return that code.
	pub fn get_status_code<'a>(&'a self, status: &'a StatusElement) -> &'a DosString {
		match status.code_source {
			CodeSource::Owned(ref code) => code,
			CodeSource::Bound(index) => self.get_status_index_code(index),
		}
	}

	/// Get the code associated with the given status.
	/// If the code of the given status is bound to the code of another status, return that code.
	pub fn get_status_code_mut<'a>(&'a mut self, status: &'a mut StatusElement) -> &'a mut DosString {
		match status.code_source {
			CodeSource::Owned(ref mut code) => code,
			CodeSource::Bound(index) => self.get_status_index_code_mut(index),
		}
	}

	/// Applies the actions in `action_result` to the board simulation state.
	///
	/// If `action_result` has a continuation, it will be repeatedly used (after applying the
	/// initial actions in `action_result`) to acquire additional actions to apply, enabling a kind
	/// of simulated board simulator mutability in the step functions, which otherwise cannot modify
	/// the simulator state while they run.
	///
	/// `current_tile_x`/`current_tile_y` represents the coordinate of the tile that is applying the
	/// actions. For example, when a boulder is pushed and the boulder applies an action to move
	/// itself, the current tile is the boulder's tile.
	/// `global_cycle` is the number of simulation steps since the start of the game.
	/// `processing_status_index` is the (optional) status index of the tile applying the actions.
	/// `accumulated_data`: see `AccumulatedActionData`.
	///
	/// Returns `ApplyActionResultReport` which contains various information about the outcomes of
	/// specific actions.
	fn apply_action_result(&mut self,
			mut current_tile_x: i16,
			mut current_tile_y: i16,
			action_result: ActionResult,
			global_cycle: usize,
			processing_status_index: Option<usize>,
			accumulated_data: &mut AccumulatedActionData) -> ApplyActionResultReport {
		//println!("{}x{}: {:?}", current_tile_x, current_tile_y, action_result);

		let mut report = ApplyActionResultReport::new();
		for action in action_result.actions {
			self.apply_action(current_tile_x, current_tile_y, action, global_cycle, processing_status_index, accumulated_data, &mut report);
		}

		if let Some(processing_status_index) = processing_status_index {
			if let Some(mut continuation) = action_result.continuation {
				loop {
					// ZZT ceases execution if a status element at an index on or below the
					// currently executing status' index is removed.
					if let Some(minimum_removed) = report.removed_status_indices.minimum() {
						if minimum_removed <= processing_status_index {
							break;
						}
					}

					//println!("Continuing: {}", processing_status_index);
					let status_element = &self.status_elements[processing_status_index];
					let continue_result = continuation.next_step(report, processing_status_index, status_element, self);

					// The current_tile_x and current_tile_y may have changed, so the
					// status.location* should be used from this point onwards. It can change eg.
					// when an object is set to #walk.
					current_tile_x = status_element.location_x as i16;
					current_tile_y = status_element.location_y as i16;

					report = ApplyActionResultReport::new();
					for action in continue_result.actions {
						self.apply_action(current_tile_x, current_tile_y, action, global_cycle, Some(processing_status_index), accumulated_data, &mut report);
					}

					if continue_result.finished {
						break;
					}
				}

				let status_element_opt = self.status_elements.get(processing_status_index);
				let finalise_actions = continuation.finalise(status_element_opt, self);

				// We don't create a new report here because it should include the information from
				// the very last set of actions applied, especially the removed_status_indices,
				// which are important to the main status processing function.
				for action in finalise_actions {
					self.apply_action(current_tile_x, current_tile_y, action, global_cycle, Some(processing_status_index), accumulated_data, &mut report);
				}
			}
		}

		report
	}

	/// Applies an individual action. This should usually be called by `apply_action_result`.
	///
	/// `current_tile_x`/`current_tile_y` represents the coordinate of the tile that is applying the
	/// action. For example, when a boulder is pushed and the boulder applies an action to move
	/// itself, the current tile is the boulder's tile.
	/// `global_cycle` is the number of simulation steps since the start of the game.
	/// `processing_status_index` is the (optional) status index of the tile applying the action.
	/// `accumulated_data`: see `AccumulatedActionData`.
	/// `report` contains various information about the outcomes of specific actions.
	pub fn apply_action(&mut self,
			current_tile_x: i16,
			current_tile_y: i16,
			action: Action,
			global_cycle: usize,
			processing_status_index: Option<usize>,
			accumulated_data: &mut AccumulatedActionData,
			report: &mut ApplyActionResultReport) {
		//println!("{}x{}: {:?}", current_tile_x, current_tile_y, action);
		match action {
			Action::SetTile{x, y, tile, status_element} => {
				self.set_tile(x, y, tile);
				let removed_indices = self.remove_status_for_pos(x, y);
				for removed_index in removed_indices {
					report.removed_status_indices.push(removed_index);
				}

				if let Some(status_element) = status_element {
					self.status_elements.push(status_element);
				}
			}
			Action::SetTileElementIdAndColour{x, y, element_id, colour} => {
				if let Some(ref mut tile) = self.get_tile_mut(x, y) {
					if let Some(element_id) = element_id {
						tile.element_id = element_id;
					}

					if let Some(colour) = colour {
						tile.colour = colour;
					}
				}
			}
			Action::SetColour{x, y, colour} => {
				if let Some(ref mut tile) = self.get_tile_mut(x, y) {
					tile.colour = colour;
				}
			}
			Action::PushTile{x, y, offset_x, offset_y} => {
				let current_tile_behaviour = self.behaviour_for_pos(current_tile_x, current_tile_y);
				let can_squash = current_tile_behaviour.can_squash();
				let push_blocked = self.push_tile(x, y, offset_x, offset_y, false, can_squash, global_cycle, processing_status_index, accumulated_data);

				if push_blocked == BlockedStatus::Blocked {
					report.move_was_blocked = BlockedStatus::Blocked;
				}
			}
			Action::MoveTile{from_x, from_y, to_x, to_y, offset_x, offset_y, check_push, is_player} => {
				if check_push {
					let current_tile_behaviour = self.behaviour_for_pos(current_tile_x, current_tile_y);
					let can_squash = current_tile_behaviour.can_squash();
					let push_blocked = self.push_tile(to_x, to_y, offset_x, offset_y, is_player, can_squash, global_cycle, processing_status_index, accumulated_data);

					if push_blocked == BlockedStatus::NotBlocked {
						self.move_tile(from_x, from_y, to_x, to_y);
					} else {
						report.move_was_blocked = BlockedStatus::Blocked;
					}
				} else {
					self.move_tile(from_x, from_y, to_x, to_y);
				}
			}
			Action::SendBoardMessage(board_message) => {
				accumulated_data.board_messages.push(board_message);
			}
			Action::SetCodeCurrentInstruction{status_index, code_current_instruction} => {
				self.status_elements[status_index].code_current_instruction = code_current_instruction;
			}
			Action::SetCode{status_index, code} => {
				*self.get_status_index_code_mut(status_index) = code;
			}
			Action::BindCodeToIndex{status_index, bind_to_index} => {
				self.status_elements[status_index].code_source = CodeSource::Bound(bind_to_index);
			}
			Action::ModifyPlayerItem{item_type, offset, require_exact_amount} => {
				if let Some(current_item_value) = item_type.get_from_world_header_mut(&mut self.world_header) {
					if offset < 0 && *current_item_value + offset < 0 {
						if require_exact_amount {
							report.take_player_item_failed = true;
						} else {
							*current_item_value = 0;
						}
					} else {
						*current_item_value += offset;
					}
				}
			}
			Action::CheckRestartOnZapped => {
				if self.board_meta_data.restart_on_zap {
					self.restart_player_on_board(&mut accumulated_data.board_messages);
				}
			}
			Action::ModifyPlayerKeys{index, value} => {
				self.world_header.player_keys[index as usize] = value;
			}
			Action::SetLeader{status_index, leader} => {
				self.status_elements[status_index].leader = leader;
			}
			Action::SetFollower{status_index, follower} => {
				self.status_elements[status_index].follower = follower;
			}
			Action::SetStep{status_index, step_x, step_y} => {
				let status_element = &mut self.status_elements[status_index];
				status_element.step_x = step_x;
				status_element.step_y = step_y;
			}
			Action::SetCycle{status_index, cycle} => {
				let status_element = &mut self.status_elements[status_index];
				status_element.cycle = cycle;
			}
			Action::OthersApplyLabelOperation{ref receiver_name_opt, ref label, ref operation, current_status_index} => {
				//println!("OthersApplyLabelOperation: {:?}, {:?}, {:?}", receiver_name_opt, label, operation);
				for status_index in 0 .. self.status_elements.len() {
					// This allows current_status_index to be None, which will send the message
					// to all statuses. This allows a push action to send a message to everything.
					// (the push action doesn't apply to any particular status, so there's no
					// "current processing status")
					if current_status_index.is_none() || status_index != current_status_index.unwrap() {
						let other_status = &self.status_elements[status_index];
						let behaviour = self.behaviour_for_pos(other_status.location_x as i16, other_status.location_y as i16);

						let mut is_matching_status = false;

						let mut parser = OopParser::new(self.get_status_code(other_status), 0);

						if !behaviour.locked(other_status) {
							if let Some(ref reciever_name) = receiver_name_opt {
								// Here, it only processes status elements with the given
								// reciever_name as their @name.
								if parser.get_name().map(|name| name.to_lower()).as_ref() == Some(&reciever_name) {
									is_matching_status = true;
								}
							} else {
								is_matching_status = true;
							}
						}

						if is_matching_status {
							let changed_pos = parser.apply_label_operation(label, *operation);

							let new_code_opt = if let Cow::Owned(new_code) = parser.code {
								Some(new_code)
							} else {
								None
							};

							if changed_pos {
								let new_pos = parser.pos;
								let other_status_mut = &mut self.status_elements[status_index];
								other_status_mut.code_current_instruction = new_pos;
							}

							if let Some(new_code) = new_code_opt {
								*self.get_status_index_code_mut(status_index) = new_code;
							}
						}
					}
				}
			}
			Action::SetStatusParam1{value, status_index} => {
				let status_element = &mut self.status_elements[status_index];
				status_element.param1 = value;
			}
			Action::SetStatusParam2{value, status_index} => {
				let status_element = &mut self.status_elements[status_index];
				status_element.param2 = value;
			}
			Action::SetStatusParam3{value, status_index} => {
				let status_element = &mut self.status_elements[status_index];
				status_element.param3 = value;
			}
			Action::SetTorchCycles(new_torch_cycles) => {
				if let Some(ref mut torch_cycles) = self.world_header.torch_cycles {
					*torch_cycles = new_torch_cycles;
				}
			}
			Action::SetEnergyCycles(new_energy_cycles) => {
				self.world_header.energy_cycles = new_energy_cycles;
			}
			Action::SetFlag(name) => {
				// Don't set the same flag twice:
				if self.world_header.last_matching_flag(name.clone()).is_none() {
					if let Some(flag_index) = self.world_header.first_empty_flag() {
						let upper_name = name.to_upper();
						self.world_header.flag_names[flag_index] = upper_name;
					}
				}
			}
			Action::ClearFlag(name) => {
				if let Some(flag_index) = self.world_header.last_matching_flag(name) {
					self.world_header.flag_names[flag_index].data.clear();
				}
			}
			Action::SetStatusLocation{x, y, status_index} => {
				let status_element = &mut self.status_elements[status_index];
				status_element.location_x = x as u8;
				status_element.location_y = y as u8;
			}
			Action::ReprocessSameStatusIndexOnRemoval => {
				report.reprocess_same_status_index_on_removal = true;
			}
			Action::CheckTimeElapsed => {
				accumulated_data.should_check_time_elapsed = true;
			}
			Action::SetAsPlayerTile{x, y} => {
				let mut player_colour = 0x1f;

				if self.world_header.energy_cycles > 0 {
					// When energised the player's background alternates between black and a colour, when
					// the colour is: LightRed, then Yellow, Blue, Cyan, Magenta, LightGray, Green.
					let colours = [0, 0xc, 0, 0xe, 0, 0x1, 0, 0x3, 0, 0x5, 0, 0x7, 0, 0xa];
					player_colour = (colours[global_cycle % colours.len()] << 4) | 0xf;
				}

				if let Some(ref mut tile) = self.get_tile_mut(x, y) {
					tile.element_id = ElementType::Player as u8;
					tile.colour = player_colour;
				}
			}
		}
	}

	/// Choose a random axis-aligned direction facing towards the player.
	pub fn seek_direction(&self, from_x: i16, from_y: i16) -> Direction {
		let (player_x, player_y) = self.get_player_location();
		let ord_x = player_x.cmp(&from_x);
		let ord_y = player_y.cmp(&from_y);

		let choose_rnd_direction = |dir_a, dir_b| {
			let mut rng = rand::thread_rng();
			let random_bool: bool = rng.gen();
			if random_bool {
				dir_a
			} else {
				dir_b
			}
		};

		let chosen_direction = match (ord_x, ord_y) {
			(Ordering::Equal, Ordering::Equal) => Direction::Idle,
			(Ordering::Equal, Ordering::Less) => Direction::North,
			(Ordering::Equal, Ordering::Greater) => Direction::South,
			(Ordering::Less, Ordering::Equal) => Direction::West,
			(Ordering::Greater, Ordering::Equal) => Direction::East,
			(Ordering::Greater,Ordering::Greater) => choose_rnd_direction(Direction::East, Direction::South),
			(Ordering::Greater,Ordering::Less) => choose_rnd_direction(Direction::East, Direction::North),
			(Ordering::Less,Ordering::Greater) => choose_rnd_direction(Direction::West, Direction::South),
			(Ordering::Less,Ordering::Less) => choose_rnd_direction(Direction::West, Direction::North),
		};

		if self.world_header.energy_cycles > 0 {
			chosen_direction.opposite()
		} else {
			chosen_direction
		}
	}
}

/// This is passed to BoardSimulator methods that deal with applying `Action`s, to collect things
/// that need to be handled outside the `BoardSimulator`.
#[derive(Clone)]
pub struct AccumulatedActionData {
	/// When true, the board time elapsed should be checked. This will be the case for every
	/// simulation step on a board that has a time limit.
	pub should_check_time_elapsed: bool,
	/// `BoardMessage`s that need to be handled outside the `BoardSimulator`.
	pub board_messages: Vec<BoardMessage>,
}

impl AccumulatedActionData {
	/// Make a new `AccumulatedActionData` that doesn't check time elapsed, and has no board
	/// messages.
	pub fn new() -> AccumulatedActionData {
		AccumulatedActionData {
			should_check_time_elapsed: false,
			board_messages: vec![],
		}
	}
}

/// To simulate a frame in a ZZT board, all you have to do is loop over the `status_elements` list
/// in order and call their respective step functions (or not, if the cycle of the status is > 1,
/// meaning it is only sometimes simulated).
///
/// When simulating a step in the `BoardSimulator`, sometimes it is necessary to pause execution of
/// that step half-way through running it. For example, if an OOP script opens a scroll, the scroll
/// will actually pause the game while executing a particular status index, and when the scroll
/// closes, it will continue the simulation step at the same status index.
///
/// This struct saves the state of a partially-executed board simulation step, so it can be returned
/// to later.
#[derive(Clone)]
pub struct BoardSimulatorStepState {
	/// The user input event from immediately before this board simulation step began. This remains
	/// the same for the entire step.
	pub event: Event,
	/// The number of steps executed since the start of the game. This remains the same for the
	/// entire step.
	pub global_cycle: usize,
	/// The current status index being processed.
	pub processing_status_index_opt: Option<usize>,
	/// The accumulated action data. See `AccumulatedActionData`.
	pub accumulated_data: AccumulatedActionData,
}

impl BoardSimulatorStepState {
	/// Make a `BoardSimulatorStepState` that will execute at status 0 (the player status).
	pub fn new(event: Event, global_cycle: usize) -> BoardSimulatorStepState {
		BoardSimulatorStepState {
			event,
			global_cycle,
			processing_status_index_opt: None,
			accumulated_data: AccumulatedActionData::new(),
		}
	}

	/// Execute a "partial step", which basically means "execute the next status' step function".
	/// This handles the possibility of a status being deleted, and may execute the same status
	/// *index* more than once, if a status with an index below the current status is removed in the
	/// process.
	/// The `accumulated_data` member of `BoardSimulatorStepState` should be considered after every
	/// partial step.
	/// `sim` is a reference to the `BoardSimulator` where the simulation takes place.
	/// Returns true when the full step is finished, and therefore `partial_step` should not be
	/// called again, and the BoardSimulatorStepState can be discarded (after dealing with the
	/// accumulated data).
	pub fn partial_step(&mut self, process_same_status: bool, sim: &mut BoardSimulator) -> bool {
		//println!("{:?}", sim.status_elements);
		// If a script opens a scroll, and the user clicks a link in that scroll, processing_status_index doesn't increment.
		let status_index = if let Some(ref mut processing_status_index) = self.processing_status_index_opt {
			if !process_same_status {
				*processing_status_index += 1;
			}
			*processing_status_index
		} else {
			self.processing_status_index_opt = Some(0);
			0
		};

		if status_index < sim.status_elements.len() {
			let action_report = self.process_status(status_index, sim);
			if let Some(ref mut processing_status_index) = self.processing_status_index_opt {
				let mut removed_below_or_at = 0;
				for removed_index in action_report.removed_status_indices.indices() {
					// This needs to be < and not <= because eg. if we are processing status #3, and
					// it removes itself and #2, ZZT will account for the removal of #2, but not #3,
					// so it subtracts only 1 (not 2), so it will process #3 again. This is probably
					// to prevent infinite loops. See FSLIME.ZZT in the tests folder.
					if *removed_index < *processing_status_index || (action_report.reprocess_same_status_index_on_removal && *removed_index == *processing_status_index) {
						removed_below_or_at += 1;
					}
				}
				*processing_status_index -= removed_below_or_at;
			}
			false
		} else {
			// We're done.
			true
		}
	}

	/// Process the status at `status_index`. This is called by `partial_step`.
	/// `sim` is a reference to the `BoardSimulator` where the simulation takes place.
	/// Returns the `ApplyActionResultReport` with various information about the latest actions that
	/// were just applied.
	fn process_status(&mut self, status_index: usize, sim: &mut BoardSimulator) -> ApplyActionResultReport {
		let mut step_result = ActionResult::do_nothing();

		let status_element = &sim.status_elements[status_index];
		let tile_x = status_element.location_x as i16;
		let tile_y = status_element.location_y as i16;

		if status_element.cycle > 0 {
			// Weird cycle calculation to match the original game. This makes it so if there are a
			// bunch of statuses with cycle 3, every third one will execute on one frame, then
			// shift across the starting index, and from that start, every third one from there
			// executes.
			if (self.global_cycle as isize - (status_index as isize % status_element.cycle as isize)) % status_element.cycle as isize == 0 {
				//println!("processing status: {} {:?}", status_index, status_element);
				let ref behaviour = sim.behaviour_for_pos(tile_x, tile_y);
				step_result = behaviour.step(self.event, &status_element, status_index, sim);
			}
		}

		sim.apply_action_result(tile_x, tile_y, step_result, self.global_cycle, Some(status_index), &mut self.accumulated_data)
	}
}
