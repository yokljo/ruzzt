use crate::board_message::*;
use crate::board_simulator::*;
use crate::event::*;
use crate::sounds::*;

use zzt_file_format::*;
use zzt_file_format::dosstring::DosString;
use std::fmt::Debug;

/// This is a description of one mutating operation to perform on the BoardSimulator.
#[derive(Debug, Clone)]
pub enum Action {
	/// Set just the tile `colour` of the tile at `x` by `y`.
	SetColour {
		x: i16,
		y: i16,
		colour: u8,
	},
	/// Set the leader value of the status with the given `status_index` (this is just for
	/// centepedes).
	SetLeader {
		status_index: usize,
		leader: i16,
	},
	/// Set the follower value of the status with the given `status_index` (this is just for
	/// centipedes).
	SetFollower {
		status_index: usize,
		follower: i16,
	},
	/// Set the step x/y values of the status with the given `status_index`.
	SetStep {
		status_index: usize,
		step_x: i16,
		step_y: i16,
	},
	/// Try to push the tile at the given `x`/`y` position by `offset_x`x`offset_y` positions.
	PushTile {
		x: i16,
		y: i16,
		offset_x: i16,
		offset_y: i16,
	},
	/// Try to move the tile at `from_x`x`from_y` to `to_x`x`to_y`, as if it were trying to move by
	/// `offset_x`x`offset_y` positions.
	/// The offset values must be separate because of things like the transporter, which can
	/// teleport a thing a long way across the board, but still only push the thing on the other
	/// side out of the way by one tile.
	MoveTile {
		from_x: i16,
		from_y: i16,
		to_x: i16,
		to_y: i16,
		offset_x: i16,
		offset_y: i16,
		// If this is true it will try and push, and not fulfill the move if the push fails.
		// Otherwise it will just move it without pushing anything.
		check_push: bool,
		is_player: bool,
	},
	/// Set the `code_current_instruction` value on the status with the given `status_index` (the
	/// index within the code string that will be parsed and executed from next).
	SetCodeCurrentInstruction {
		status_index: usize,
		code_current_instruction: i16
	},
	/// Set the code of the current status being processed.
	/// If the status binds to another status' code, THAT status' code will be set instead.
	SetCode{
		status_index: usize,
		code: DosString,
	},
	/// Setup the code of the current status being processed to bind to the code of a different
	/// status, via its status index.
	BindCodeToIndex{
		status_index: usize,
		bind_to_index: usize,
	},
	/// Replace the tile and statuses at the given `x`x`y` position with the given `tile` and
	/// `status_element` (if it's not None). Note that `status_element`'s `location_x` and
	/// `location_y` values will be overridden with `x` and `y`.
	SetTile {
		x: i16,
		y: i16,
		tile: BoardTile,
		status_element: Option<StatusElement>,
	},
	/// Change the `element_id` (when not None) and the `colour` (when not None) value of the tile
	/// at the given `x`x`y` position.
	SetTileElementIdAndColour {
		x: i16,
		y: i16,
		element_id: Option<u8>,
		colour: Option<u8>,
	},
	/// Set the tile at the given `x`x`y` position to be the player. This has its own action because
	/// the player tile's colour is the only thing that depends on the `global_cycle`.
	SetAsPlayerTile {
		x: i16,
		y: i16,
	},
	/// Apply a label operation to every other status element than the one who applied the
	/// operation. The type of operation could be a `#send`, `#zap` or `#restore`, depedinging on
	/// the value of `operation`.
	/// If the `receiver_name_opt` is not None, it will only send the message to objects with that
	/// name.
	OthersApplyLabelOperation {
		current_status_index: Option<usize>,
		receiver_name_opt: Option<DosString>,
		label: DosString,
		operation: LabelOperation,
	},
	/// Send the given board message, which will be applied after the current status is finished
	/// being processed. Board messages are sent all the way out to the front-end, which are then
	/// intercepted and/or passed back into `ZztEngine::process_board_message`.
	SendBoardMessage(BoardMessage),
	/// Give or take `offset` amount of the item of the type `item_type` from the player.
	/// If `require_exact_amount` is true then the item will only be taken if the player has enough
	/// of it. If there is an `ActionContinuation` to run after this action is applied, it will be
	/// informed of this failure to take an item.
	ModifyPlayerItem{
		item_type: PlayerItemType,
		offset: i16,
		/// When this is true, if the offset is going to take more than the current value of the
		/// item, then it will fail to do so, and respond with `Response::TakePlayerItemFailed`.
		/// Note that values will never go below zero. This will only matter for negative offsets.
		require_exact_amount: bool,
	},
	/// Give (when `value` is true) or take (when `value` is false) the key with the given key
	// `index` from the player.
	ModifyPlayerKeys{
		index: u8,
		value: bool,
	},
	/// Set the player's current torch cycles (the number of cycles until a lit torch runs out).
	SetTorchCycles(i16),
	/// Set the player's current energy cycles (the number of cycles until an energizer runs out).
	SetEnergyCycles(i16),
	/// `#set` the flag with the given name.
	SetFlag(DosString),
	/// `#clear` the flag with the given name.
	ClearFlag(DosString),
	/// Set the `location_x` and `location_y` values for the status with the given `status_index`.
	SetStatusLocation {
		x: i16,
		y: i16,
		status_index: usize,
	},
	/// Set the `param1` `value` for the status with the given `status_index`.
	SetStatusParam1{value: u8, status_index: usize},
	/// Set the `param2` `value` for the status with the given `status_index`.
	SetStatusParam2{value: u8, status_index: usize},
	/// Set the `param3` `value` for the status with the given `status_index`.
	SetStatusParam3{value: u8, status_index: usize},
	/// This sets the reprocess_same_status_index_on_removal flag on the action report.
	ReprocessSameStatusIndexOnRemoval,
	/// Applied when the player is hurt or the board timer runs out (the player is also hurt when
	/// this happens), to check if the board is supposed to reset the player's location, and do so
	/// if it is.
	CheckRestartOnZapped,
	/// This action is sent by the player when the centiseconds passed should be checked against the
	/// time_passed_ticks value in the world header to see if a second has passed, and it should
	/// increment the time_passed value in the world header.
	CheckTimeElapsed,
	/// Set the cycle of the given status index (the number of game steps between each
	/// time the status is processed).
	SetCycle{status_index: usize, cycle: i16},
}

/// Player items are all integers that can be added to or subtracted from. This enum describes one
/// of those items.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PlayerItemType {
	Ammo,
	Torches,
	Gems,
	Health,
	Score,
	// NOTE: Modifying the time item actually modifies time_passed, so the OOP actually
	// negates the argument before trying to modify the time.
	Time,
}

impl PlayerItemType {
	/// Get the value of a particular item from the world header.
	pub fn get_from_world_header(self, world_header: &WorldHeader) -> Option<i16> {
		match self {
			PlayerItemType::Ammo => Some(world_header.player_ammo),
			PlayerItemType::Torches => world_header.player_torches,
			PlayerItemType::Gems => Some(world_header.player_gems),
			PlayerItemType::Health => Some(world_header.player_health),
			PlayerItemType::Score => Some(world_header.player_score),
			PlayerItemType::Time => Some(world_header.time_passed),
		}
	}

	/// Get the value of a particular item from the world header as a mutable reference so it can
	/// be directly modified.
	pub fn get_from_world_header_mut(self, world_header: &mut WorldHeader) -> Option<&mut i16> {
		match self {
			PlayerItemType::Ammo => Some(&mut world_header.player_ammo),
			PlayerItemType::Torches => world_header.player_torches.as_mut(),
			PlayerItemType::Gems => Some(&mut world_header.player_gems),
			PlayerItemType::Health => Some(&mut world_header.player_health),
			PlayerItemType::Score => Some(&mut world_header.player_score),
			PlayerItemType::Time => Some(&mut world_header.time_passed),
		}
	}
}

/// The particular operation to perform when working with object labels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LabelOperation {
	/// Jump to a given label.
	Jump,
	/// Zap a given label (replace the `:` character with a `'` character).
	Zap,
	/// This is for when you `#restore [object_name:]label_name`. It restores (replaces the `'`
	/// character with a `:` character) the first label matching `label_name`. For all following
	/// matches in the code, if there is an `object_name`, it will restore any labels matching that
	/// object name, otherwise, it will continue matching against `label_name`.
	/// For example, running the following:
	/// status1: `#restore a:b`
	/// status2: `@a\r'b\r'b\r'a\r`
	/// The status2 code will become `@a\r:b\r'b\r:a\r`.
	RestoreZztStyle,
}

/// Whether some movement action was blocked or not.
#[derive(Debug, Clone, PartialEq)]
pub enum BlockedStatus {
	Blocked,
	NotBlocked,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DamageType {
	Shot{by_player: bool},
	Bombed,
	Other,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DamageResult {
	None,
	Died,
}

/// Helper for keeping track of a list on indices, and the minimum value within that list.
#[derive(Clone)]
pub struct StatusIndicesWithMinimum {
	indices: Vec<usize>,
	minimum: Option<usize>,
}

impl StatusIndicesWithMinimum {
	pub fn new() -> StatusIndicesWithMinimum {
		StatusIndicesWithMinimum {
			indices: vec![],
			minimum: None,
		}
	}

	/// Add an index to the list, while updating the minimum if necessary.
	pub fn push(&mut self, add_index: usize) {
		self.indices.push(add_index);
		match self.minimum {
			Some(ref mut index) => {
				if *index > add_index {
					*index = add_index;
				}
			}
			None => {
				self.minimum = Some(add_index);
			}
		}
	}

	/// The list of `push()`ed indices.
	pub fn indices(&self) -> &Vec<usize> { &self.indices }
	/// The minimum value of the indices list, or None if there's nothing in the list.
	pub fn minimum(&self) -> Option<usize> { self.minimum }
}

/// This is information that is passed to ActionContinuation::next_step about the actions that were
/// applied by its last invocation.
pub struct ApplyActionResultReport {
	/// If one of the last actions that were applied tried to move somewhere, but was blocked, then
	/// this represents that. Otherwise it will just be NotBlocked.
	pub move_was_blocked: BlockedStatus,
	/// If one of the last actions that were applied tried to take some item from a player, but the
	/// player didn't have enough of that item to take, this will be true. Otherwise it will just be
	/// false.
	pub take_player_item_failed: bool,
	/// Any action that was just applied that caused a status to be removed, the index of that
	/// removed status is added to this list.
	pub removed_status_indices: StatusIndicesWithMinimum,
	/// If this is true, then if the current status is removed, it will reprocess it again. In other
	/// words, it will subtract one if removed_status_indices contains the current processing status
	/// index.
	pub reprocess_same_status_index_on_removal: bool,
}

impl ApplyActionResultReport {
	/// Make a new default `ApplyActionResultReport`.
	pub fn new() -> ApplyActionResultReport {
		ApplyActionResultReport {
			move_was_blocked: BlockedStatus::NotBlocked,
			take_player_item_failed: false,
			removed_status_indices: StatusIndicesWithMinimum::new(),
			reprocess_same_status_index_on_removal: false,
		}
	}
}

/// The result of the `Behaviour::push()` function.
#[derive(Debug)]
pub struct PushResult {
	/// Whether the push attempt was blocked, and so the thing pushing it shouldn't assume that the
	/// spot is empty.
	pub blocked: BlockedStatus,
	/// The actions to apply due to the push action.
	pub action_result: ActionResult,
}

impl PushResult {
	/// Helper constructor for something that does nothing and blocks the way when pushed.
	pub fn do_nothing_blocked() -> PushResult {
		PushResult {
			blocked: BlockedStatus::Blocked,
			action_result: ActionResult::do_nothing(),
		}
	}

	/// Helper constructor for something that does nothing but does not block the way when pushed,
	/// eg. an empty space or a fake wall.
	pub fn do_nothing_not_blocked() -> PushResult {
		PushResult {
			blocked: BlockedStatus::NotBlocked,
			action_result: ActionResult::do_nothing(),
		}
	}
}

/// Standard result for specifying what mutating actions should be applied after a Behaviour
/// function is called.
#[derive(Debug)]
pub struct ActionResult {
	/// List of mutating actions to apply to the BoardSimulator.
	pub actions: Vec<Action>,
	/// If this is not None, it will be the continuation object to use to keep executing logic after
	/// applying `actions`. See `ActionContinuation`.
	pub continuation: Option<Box<ActionContinuation>>,
}

impl ActionResult {
	/// Helper constructor for making an `ActionResult` with some `actions` and no `continuation`.
	pub fn with_actions(actions: Vec<Action>) -> ActionResult {
		ActionResult {
			actions,
			continuation: None,
		}
	}

	/// Helper constructor for making an `ActionResult` that does nothing.
	pub fn do_nothing() -> ActionResult {
		ActionResult {
			actions: vec![],
			continuation: None,
		}
	}
}

/// Result of `ActionContinuation::next_step()`.
pub struct ActionContinuationResult {
	/// List of mutating actions to apply to the BoardSimulator.
	pub actions: Vec<Action>,
	/// When this is false, after applying the `actions`, the `ActionContinuation::next_step`
	/// function will be called again.
	pub finished: bool,
}

/// If a behaviour has to mutate the world then keep processing, it needs to unborrow everything so
/// the world state can be mutated, then come back. This trait represents the current running state
/// of one of those functions that needs to mutate game state and come back. The `next_step` method
/// will be called after applying all actions returned from the behavour method, then the actions
/// returned by `next_step` will be applied, and next_step invoked again.
pub trait ActionContinuation: Debug {
	/// This is called after applying some mutating actions to BoardSimulator, and will continue to
	/// be called until it returns `finished` as true in the `ActionContinuationResult`.
	fn next_step(&mut self, apply_action_report: ApplyActionResultReport, status_index: usize, status: &StatusElement, sim: &BoardSimulator) -> ActionContinuationResult;

	/// This is guaranteed to be called as the very last operation on a continuation, so some final
	/// actions can be applied. The ApplyActionResultReport generated by these actions will be
	/// returned from BoardSimulator::apply_action_result.
	fn finalise(&mut self, _status_opt: Option<&StatusElement>, _sim: &BoardSimulator) -> Vec<Action> {
		vec![]
	}
}

pub fn default_damage_impl(is_destructable: bool, x: i16, y: i16, damage_type: DamageType, sim: &BoardSimulator, actions: &mut Vec<Action>) -> DamageResult {
	let should_die = match damage_type {
		DamageType::Shot{..} => {
			if is_destructable {
				actions.push(Action::SendBoardMessage(BoardMessage::PlaySoundArray(process_notes_string(b"t-c"), SoundPriority::Level(2))));
			}
			is_destructable
		}
		DamageType::Bombed => {
			let has_status = sim.get_first_status_for_pos(x, y).is_some();
			if !has_status && is_destructable {
				true
			} else {
				false
			}
		}
		DamageType::Other => {
			false
		}
	};

	if should_die {
		if let Some(tile) = sim.get_tile(x, y) {
			actions.push(Action::SetTile {
				x,
				y,
				tile: BoardTile {
					element_id: ElementType::Empty as u8,
					colour: tile.colour,
				},
				status_element: None,
			});
		}
		DamageResult::Died
	} else {
		DamageResult::None
	}
}

/// A description of the Behaviour of a particular element type.
pub trait Behaviour: Debug {
	/// Called every time a status element cycles.
	fn step(&self, _event: Event, _status: &StatusElement, _status_index: usize, _sim: &BoardSimulator) -> ActionResult {
		ActionResult {
			actions: vec![],
			continuation: None,
		}
	}

	/// This is called when applying certain movement actions to try and push something out of the
	/// way so that another thing can move there. See `PushResult`.
	/// `x` and `y` are the location of the tile being pushed.
	/// Using `push_off_x` and `push_off_y` instead of direction, because a pusher can push a
	/// boulder two away, by two, and that will propagate along a line of boulders.
	/// `is_player` is true if this is being pushed by a player.
	fn push(&self, _x: i16, _y: i16, _push_off_x: i16, _push_off_y: i16, _is_player: bool, _sim: &BoardSimulator) -> PushResult {
		PushResult::do_nothing_blocked()
	}

	/// The value for the BLOCKED #if flag. This is also used for other things like walking objects
	/// to see if they are allowed to walk (walking objects don't push boulders).
	/// `is_player` should be true to check if this is blocked for the player.
	fn blocked(&self, _is_player: bool) -> BlockedStatus {
		BlockedStatus::Blocked
	}

	/// Whether this type blocks the path of bullets.
	fn blocked_for_bullets(&self) -> BlockedStatus {
		self.blocked(false)
	}

	/// Whether jump, zap or restore label actions are allowed to be applied on this element.
	/// In other words, this becomes true when `#lock` is invoked in OOP, and becomes false when
	/// `#unlock` is invoked in OOP.
	fn locked(&self, _status: &StatusElement) -> bool {
		false
	}

	fn damage(&self, x: i16, y: i16, damage_type: DamageType, sim: &BoardSimulator, actions: &mut Vec<Action>) -> DamageResult {
		default_damage_impl(self.destructable(), x, y, damage_type, sim, actions)
	}

	/// If this is true:
	/// - the player can fire a bullet at this tile when it is right next to it
	///   (so a bullet can't actually appear)
	/// - a bomb can blow this up
	/// - a blinking wall can destroy it
	fn destructable(&self) -> bool {
		false
	}

	/// This returns true if a conveyor can move this type.
	fn conveyable(&self) -> bool {
		false
	}

	/// This is true if a player can push it onto a type that `can_be_squashed`.
	fn can_squash(&self) -> bool {
		false
	}

	/// This is true if a player can push a `can_squash` type on top of this type.
	fn can_be_squashed(&self) -> bool {
		false
	}
}

/// A Behaviour for things that don't explicitly have one, such as out-of-bounds positions.
#[derive(Debug, Clone)]
pub struct DefaultBehaviour;

impl Behaviour for DefaultBehaviour {}
