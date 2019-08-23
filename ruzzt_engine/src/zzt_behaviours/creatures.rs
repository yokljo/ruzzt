use crate::behaviour::*;
use crate::board_simulator::*;
use crate::event::*;
use crate::oop_parser::*;

use rand::Rng;

use zzt_file_format::*;
use zzt_file_format::dosstring::DosString;

use crate::zzt_behaviours::monster_interactions::*;

/*
param1 is the "sensitivity", which is the opposite of what you think - 0 means very sensitive, 8
means not very sensitive.
When param1 is 8, the bear will only detect when either the x or y is the same as the player's x/y.
As param1 gets bigger, the absolute difference from this X/Y to the player's X/Y for the ebar to
start moving gets bigger too. Eg. If param1 is 0, the player can be 8 away from the bear on either
axis.

When the bear is moving towards the player, it will always move on the X axis first, and only move
on the Y axis if X is equal to the player's X.

Breakable walls are anti-bears, and they turn into a sigularity when they collide.
*/
#[derive(Debug, Clone)]
pub struct BearBehaviour;

impl Behaviour for BearBehaviour {
	fn step(&self, _event: Event, status: &StatusElement, _status_index: usize, sim: &BoardSimulator) -> ActionResult {
		let mut actions = vec![];
		let (player_x, player_y) = sim.get_player_location();

		let diff_x = status.location_x as i16 - player_x;
		let diff_y = status.location_y as i16 - player_y;

		let allowed_diff = (8 - status.param1) as i16;
		if diff_x.abs() <= allowed_diff || diff_y.abs() <= allowed_diff {
			let off_x;
			let off_y;

			if diff_x == 0 {
				off_x = 0;
				off_y = if diff_y > 0 { -1 } else { 1 };
			} else {
				off_x = if diff_x > 0 { -1 } else { 1 };
				off_y = 0;
			}

			let dest_x = status.location_x as i16 + off_x;
			let dest_y = status.location_y as i16 + off_y;
			if sim.has_player_at_location(dest_x, dest_y) {
				add_monster_touch_player_actions(status.location_x as i16, status.location_y as i16, &mut actions, sim);
			} else {
				if let Some(tile) = sim.get_tile(dest_x, dest_y) {
					if tile.element_id == ElementType::Breakable as u8 {
						actions.push(Action::SetTile {
							x: status.location_x as i16,
							y: status.location_y as i16,
							tile: BoardTile { element_id: status.under_element_id, colour: status.under_colour },
							status_element: None,
						});
						actions.push(Action::SetTile {
							x: dest_x,
							y: dest_y,
							tile: BoardTile { element_id: ElementType::Empty as u8, colour: 0 },
							status_element: None,
						});
					} else {
						let dest_behaviour = sim.behaviour_for_pos(dest_x, dest_y);
						if dest_behaviour.blocked(false) == BlockedStatus::NotBlocked {
							actions.push(Action::MoveTile {
								from_x: status.location_x as i16,
								from_y: status.location_y as i16,
								to_x: status.location_x as i16 + off_x,
								to_y: status.location_y as i16 + off_y,
								offset_x: off_x,
								offset_y: off_y,
								check_push: true,
								is_player: false,
							});
						}
					}
				}
			}
		}

		ActionResult::with_actions(actions)
	}

	fn push(&self, x: i16, y: i16, _push_off_x: i16, _push_off_y: i16, is_player: bool, sim: &BoardSimulator) -> PushResult {
		monster_push(x, y, is_player, sim)
	}

	fn damage(&self, x: i16, y: i16, damage_type: DamageType, sim: &BoardSimulator, actions: &mut Vec<Action>) -> DamageResult {
		monster_damage(self, x, y, damage_type, sim, actions)
	}

	fn destructable(&self) -> bool {
		true
	}
}

/*
Intelligence is param1 (1-9 in the editor = 0-8 for param1)
Resting time is param2 (1-9 in the editor = 0-8 for param2)
1 intelligence always chooses a random direction
9 intelligence always seeks the player
1 resting time will move most of the time, but also change direction/idle more erratically.
9 resting time will stop for longer periods, but will also move longer distances at a time.
The step value always determines the movement vector, and they will tend to walk in straight lines.
They don't have to stop to change direction.
When resting time is 1, it can still move in stright lines, so it doesn't change direction all the
time.
*/
#[derive(Debug, Clone)]
pub struct RuffianBehaviour;

impl Behaviour for RuffianBehaviour {
	fn step(&self, _event: Event, status: &StatusElement, status_index: usize, sim: &BoardSimulator) -> ActionResult {
		let mut actions = vec![];

		let mut step_x = status.step_x;
		let mut step_y = status.step_y;

		let (player_x, player_y) = sim.get_player_location();

		let mut do_move_tile = true;

		let mut rng = rand::thread_rng();
		if step_x == 0 && step_y == 0 {
			if status.param2 + 8 <= rng.gen_range(0, 17) {
				if status.param1 >= rng.gen_range(0, 9) {
					let (seek_x, seek_y) = sim.seek_direction(status.location_x as i16, status.location_y as i16).to_offset();
					step_x = seek_x;
					step_y = seek_y;
				} else {
					let (rand_step_x, rand_step_y) = sim.get_random_step();
					step_x = rand_step_x;
					step_y = rand_step_y;
				}
			}

			do_move_tile = false;
		} else {
			if status.location_x as i16 == player_x || status.location_y as i16 == player_y {
				if status.param1 >= rng.gen_range(0, 9) {
					let (seek_x, seek_y) = sim.seek_direction(status.location_x as i16, status.location_y as i16).to_offset();
					step_x = seek_x;
					step_y = seek_y;
				} else {
					// Don't change direction if aligned with the player and not seeking.
				}
			}
		}

		if step_x != status.step_x || step_y != status.step_y {
			actions.push(Action::SetStep {
				status_index,
				step_x,
				step_y,
			});
		}

		if do_move_tile {
			let dest_x = status.location_x as i16 + step_x;
			let dest_y = status.location_y as i16 + step_y;

			if sim.has_player_at_location(dest_x, dest_y) {
				add_monster_touch_player_actions(status.location_x as i16, status.location_y as i16, &mut actions, sim);
			} else {
				let dest_behaviour = sim.behaviour_for_pos(dest_x, dest_y);
				if dest_behaviour.blocked(false) == BlockedStatus::NotBlocked {
					actions.push(Action::MoveTile {
						from_x: status.location_x as i16,
						from_y: status.location_y as i16,
						to_x: status.location_x as i16 + step_x,
						to_y: status.location_y as i16 + step_y,
						offset_x: step_x,
						offset_y: step_y,
						check_push: true,
						is_player: false,
					});

					if status.param2 + 8 <= rng.gen_range(0, 17) {
						actions.push(Action::SetStep {
							status_index,
							step_x: 0,
							step_y: 0,
						});
					}
				} else {
					actions.push(Action::SetStep {
						status_index,
						step_x: 0,
						step_y: 0,
					});
				}
			}
		}

		ActionResult::with_actions(actions)
	}

	fn push(&self, x: i16, y: i16, _push_off_x: i16, _push_off_y: i16, is_player: bool, sim: &BoardSimulator) -> PushResult {
		monster_push(x, y, is_player, sim)
	}

	fn damage(&self, x: i16, y: i16, damage_type: DamageType, sim: &BoardSimulator, actions: &mut Vec<Action>) -> DamageResult {
		monster_damage(self, x, y, damage_type, sim, actions)
	}

	fn destructable(&self) -> bool {
		true
	}
}

#[derive(Debug, Clone)]
pub struct ObjectBehaviour;

impl Behaviour for ObjectBehaviour {
	fn push(&self, x: i16, y: i16, _push_off_x: i16, _push_off_y: i16, is_player: bool, sim: &BoardSimulator) -> PushResult {
		let mut actions = vec![];
		if is_player {
			if let Some((status_index, status)) = sim.get_first_status_for_pos(x, y) {
				if !self.locked(status) {
					let parser = OopParser::new(&sim.get_status_code(status), status.code_current_instruction);
					if let Some(touch_label_pos) = parser.find_label(&DosString::from_slice(b"touch")) {
						//println!("Finding touch: {}", touch_label_pos);
						actions.push(Action::SetCodeCurrentInstruction {
							status_index,
							code_current_instruction: touch_label_pos,
						});
					}
				}
			}
		}
		PushResult {
			blocked: BlockedStatus::Blocked,
			action_result: ActionResult::with_actions(actions),
		}
	}

	fn damage(&self, x: i16, y: i16, damage_type: DamageType, sim: &BoardSimulator, actions: &mut Vec<Action>) -> DamageResult {
		if let Some((status_index, ref status)) = sim.get_first_status_for_pos(x, y) {
			if !self.locked(status) {
				let parser = OopParser::new(&sim.get_status_code(status), status.code_current_instruction);
				match damage_type {
					DamageType::Bombed => {
						if let Some(label_pos) = parser.find_label(&DosString::from_slice(b"bombed")) {
							actions.push(Action::SetCodeCurrentInstruction {
								status_index,
								code_current_instruction: label_pos,
							});
						}
					}
					DamageType::Shot{..} => {
						if let Some(label_pos) = parser.find_label(&DosString::from_slice(b"shot")) {
							actions.push(Action::SetCodeCurrentInstruction {
								status_index,
								code_current_instruction: label_pos,
							});
						}
					}
					_ => {}
				}
			}
			DamageResult::None
		} else {
			default_damage_impl(self.destructable(), x, y, damage_type, sim, actions)
		}
	}

	fn step(&self, _event: Event, status: &StatusElement, status_index: usize, sim: &BoardSimulator) -> ActionResult {
		let mut actions = vec![];

		let parser = OopParser::new(&sim.get_status_code(status), status.code_current_instruction);

		// Make object walk.
		if status.step_x != 0 || status.step_y != 0 {
			let dest_behaviour = sim.behaviour_for_pos(status.location_x as i16 + status.step_x, status.location_y as i16 + status.step_y);
			if dest_behaviour.blocked(false) == BlockedStatus::Blocked {
				if !self.locked(status) {
					if let Some(thud_label_pos) = parser.find_label(&DosString::from_slice(b"thud")) {
						println!("Finding thud: {}", thud_label_pos);
						actions.insert(0, Action::SetCodeCurrentInstruction{status_index, code_current_instruction: thud_label_pos});
					}
				}
			} else {
				actions.push(Action::MoveTile {
					from_x: status.location_x as i16,
					from_y: status.location_y as i16,
					to_x: status.location_x as i16 + status.step_x,
					to_y: status.location_y as i16 + status.step_y,
					offset_x: status.step_x,
					offset_y: status.step_y,
					// It can't be blocked at this point, so don't check.
					check_push: false,
					is_player: false,
				});
			}
		}

		let continuation: Option<Box<dyn ActionContinuation>> = Some(Box::new(OopExecutionState::new(false, None)));

		ActionResult {
			actions,
			continuation,
		}
	}

	fn locked(&self, status: &StatusElement) -> bool {
		status.param2 > 0
	}
}

/*
param2 is is the movement speed. param1 increments from 0. When it steps and param1 == param2, then
it checks 4 adjacent positions to see if they are not blocked (as in blocked for Object walking.
ie. it doesn't attempt to push anything) then it spawns a new slime in those non-blocked positions
with param1 == 0, and replaces itself with a breakable.
If the player tries to push the slime, it will immediately turn into a breakable, and that's it.
The player is never hurt by contact with slime.
*/
#[derive(Debug, Clone)]
pub struct SlimeBehaviour;

impl Behaviour for SlimeBehaviour {
	fn step(&self, _event: Event, status: &StatusElement, status_index: usize, sim: &BoardSimulator) -> ActionResult {
		let mut actions = vec![];

		if status.param1 == status.param2 {
			let slime_x = status.location_x as i16;
			let slime_y = status.location_y as i16;
			if let Some(slime_tile) = sim.get_tile(slime_x, slime_y) {
				actions.push(Action::SetTile {
					x: slime_x,
					y: slime_y,
					tile: BoardTile {
						element_id: ElementType::Breakable as u8,
						colour: slime_tile.colour,
					},
					status_element: None,
				});

				// It inserts the slimes in top, bottom, left, right order.
				for (off_x, off_y) in &[(0, -1), (0, 1), (-1, 0), (1, 0)] {
					let adj_x = slime_x + off_x;
					let adj_y = slime_y + off_y;
					if sim.behaviour_for_pos(adj_x, adj_y).blocked(false) == BlockedStatus::NotBlocked {
						actions.push(Action::SetTile {
							x: adj_x,
							y: adj_y,
							tile: slime_tile,
							status_element: Some(StatusElement {
								location_x: adj_x as u8,
								location_y: adj_y as u8,
								param1: 0,
								.. status.clone()
							}),
						});
					}
				}
			}
		} else {
			actions.push(Action::SetStatusParam1{value: status.param1 + 1, status_index});
		}

		ActionResult::with_actions(actions)
	}

	fn push(&self, _x: i16, _y: i16, _push_off_x: i16, _push_off_y: i16, _is_player: bool, _sim: &BoardSimulator) -> PushResult {
		PushResult::do_nothing_blocked()
	}

	fn destructable(&self) -> bool {
		false
	}
}

#[derive(Debug, Clone)]
pub struct SharkBehaviour;

impl Behaviour for SharkBehaviour {
	fn step(&self, _event: Event, status: &StatusElement, _status_index: usize, sim: &BoardSimulator) -> ActionResult {
		let mut actions = vec![];
		let mut rng = rand::thread_rng();

		let off_x;
		let off_y;

		// If param1 (intelligence) is 0, then it should always randomise, and when 8 it
		// should ALMOST always seek.
		let should_randomise: bool = rng.gen_range(0, 9) >= status.param1;
		if should_randomise {
			let (rand_step_x, rand_step_y) = sim.get_random_step();
			off_x = rand_step_x;
			off_y = rand_step_y;
		} else {
			let (seek_x, seek_y) = sim.seek_direction(status.location_x as i16, status.location_y as i16).to_offset();
			off_x = seek_x;
			off_y = seek_y;
		}

		let dest_x = status.location_x as i16 + off_x;
		let dest_y = status.location_y as i16 + off_y;
		if sim.has_player_at_location(dest_x, dest_y) {
			add_monster_touch_player_actions(status.location_x as i16, status.location_y as i16, &mut actions, sim);
		} else {
			if let Some(tile) = sim.get_tile(dest_x, dest_y) {
				if tile.element_id == ElementType::Water as u8 {
					actions.push(Action::SetColour {
						x: status.location_x as i16,
						y: status.location_y as i16,
						colour: 0x77,
					});
					actions.push(Action::MoveTile {
						from_x: status.location_x as i16,
						from_y: status.location_y as i16,
						to_x: status.location_x as i16 + off_x,
						to_y: status.location_y as i16 + off_y,
						offset_x: off_x,
						offset_y: off_y,
						check_push: false,
						is_player: false,
					});
				}
			}
		}

		ActionResult::with_actions(actions)
	}

	fn push(&self, x: i16, y: i16, _push_off_x: i16, _push_off_y: i16, is_player: bool, sim: &BoardSimulator) -> PushResult {
		monster_push(x, y, is_player, sim)
	}

	fn damage(&self, x: i16, y: i16, damage_type: DamageType, sim: &BoardSimulator, actions: &mut Vec<Action>) -> DamageResult {
		// TODO: Check if this actually works. Maybe return died if the under_element_id is Empty.
		monster_damage(self, x, y, damage_type, sim, actions)
	}
}

#[derive(Debug, Clone)]
pub struct SpinningGunBehaviour;

impl Behaviour for SpinningGunBehaviour {
	fn step(&self, _event: Event, status: &StatusElement, _status_index: usize, sim: &BoardSimulator) -> ActionResult {
		// param1 is the intelligence (0-8)
		// param2 is a combination between the firing rate and the firing type: the first 7 bits are
		// the rate and the last bit is the firing type (0 = bullets, 1 = stars)

		let mut actions = vec![];

		let firing_rate = status.param2 & 0b01111111;
		let shoot_stars = (status.param2 & 0b10000000) != 0;

		let mut rng = rand::thread_rng();

		let try_shoot_bullet = firing_rate > rng.gen_range(0, 9);
		if try_shoot_bullet {
			let shoot_step_x;
			let shoot_step_y;

			// If param1 (intelligence) is 0, then it should always randomise, and when 8 it
			// should ALWAYS shoot towards the player.
			let should_randomise: bool = rng.gen_range(0, 9) > status.param1;
			if should_randomise {
				let (rand_step_x, rand_step_y) = sim.get_random_step();
				shoot_step_x = rand_step_x;
				shoot_step_y = rand_step_y;
			} else {
				let (player_x, player_y) = sim.get_player_location();

				let diff_x = status.location_x as i16 - player_x;
				let diff_y = status.location_y as i16 - player_y;
				let allowed_diff = 2 as i16;
				if diff_x.abs() <= allowed_diff || diff_y.abs() <= allowed_diff {
					if diff_y.abs() >= diff_x.abs() {
						// Shoot preferentially in the Y axis.
						shoot_step_x = 0;
						shoot_step_y = if diff_y > 0 { -1 } else { 1 };
					} else {
						shoot_step_x = if diff_x > 0 { -1 } else { 1 };
						shoot_step_y = 0;
					}
				} else {
					shoot_step_x = 0;
					shoot_step_y = 0;
				}
			}

			if shoot_step_x != 0 || shoot_step_y != 0 {
				let shoot_x = status.location_x as i16 + shoot_step_x;
				let shoot_y = status.location_y as i16 + shoot_step_y;

				sim.make_shoot_actions(shoot_x, shoot_y, shoot_step_x, shoot_step_y, shoot_stars, false, &mut actions);
			}
		}

		ActionResult::with_actions(actions)
	}
}

#[derive(Debug, Clone)]
pub struct PusherBehaviour;

impl Behaviour for PusherBehaviour {
	fn step(&self, _event: Event, status: &StatusElement, _status_index: usize, _sim: &BoardSimulator) -> ActionResult {
		ActionResult::with_actions(vec![
			Action::MoveTile {
				from_x: status.location_x as i16,
				from_y: status.location_y as i16,
				to_x: status.location_x as i16 + status.step_x,
				to_y: status.location_y as i16 + status.step_y,
				offset_x: status.step_x,
				offset_y: status.step_y,
				check_push: true,
				is_player: false,
			}
		])
	}
}

#[derive(Debug, Clone)]
pub struct LionBehaviour;

impl Behaviour for LionBehaviour {
	fn step(&self, _event: Event, status: &StatusElement, _status_index: usize, sim: &BoardSimulator) -> ActionResult {
		let mut actions = vec![];
		let mut rng = rand::thread_rng();

		let off_x;
		let off_y;

		// If param1 (intelligence) is 0, then it should always randomise, and when 8 it
		// should ALMOST always seek.
		let should_randomise: bool = rng.gen_range(0, 9) >= status.param1;
		if should_randomise {
			let (rand_step_x, rand_step_y) = sim.get_random_step();
			off_x = rand_step_x;
			off_y = rand_step_y;
		} else {
			let (seek_x, seek_y) = sim.seek_direction(status.location_x as i16, status.location_y as i16).to_offset();
			off_x = seek_x;
			off_y = seek_y;
		}

		let dest_x = status.location_x as i16 + off_x;
		let dest_y = status.location_y as i16 + off_y;
		if sim.has_player_at_location(dest_x, dest_y) {
			add_monster_touch_player_actions(status.location_x as i16, status.location_y as i16, &mut actions, sim);
		} else {
			let dest_behaviour = sim.behaviour_for_pos(dest_x, dest_y);
			if dest_behaviour.blocked(false) == BlockedStatus::NotBlocked {
				actions.push(Action::MoveTile {
					from_x: status.location_x as i16,
					from_y: status.location_y as i16,
					to_x: status.location_x as i16 + off_x,
					to_y: status.location_y as i16 + off_y,
					offset_x: off_x,
					offset_y: off_y,
					check_push: true,
					is_player: false,
				});
			}
		}

		ActionResult::with_actions(actions)
	}

	fn push(&self, x: i16, y: i16, _push_off_x: i16, _push_off_y: i16, is_player: bool, sim: &BoardSimulator) -> PushResult {
		monster_push(x, y, is_player, sim)
	}

	fn damage(&self, x: i16, y: i16, damage_type: DamageType, sim: &BoardSimulator, actions: &mut Vec<Action>) -> DamageResult {
		monster_damage(self, x, y, damage_type, sim, actions)
	}

	fn destructable(&self) -> bool {
		true
	}
}

/*
param1 is the intelligence (0 is random movement, 8 is ALMOST always seek).
param2 is two things:
the right 7 bits are the "firing rate" (0 is really slow, 8 is really fast).
the left bit is 0 when it should shoot bullets, and 1 when it should shoot stars.
*/
#[derive(Debug, Clone)]
pub struct TigerBehaviour;

impl Behaviour for TigerBehaviour {
	fn step(&self, _event: Event, status: &StatusElement, _status_index: usize, sim: &BoardSimulator) -> ActionResult {
		let mut actions = vec![];
		let mut rng = rand::thread_rng();

		let (player_x, player_y) = sim.get_player_location();

		let shot_bullet: bool;

		let diff_x = status.location_x as i16 - player_x;
		let diff_y = status.location_y as i16 - player_y;
		let allowed_diff = 2 as i16;
		if diff_x.abs() <= allowed_diff || diff_y.abs() <= allowed_diff {
			let mut rng = rand::thread_rng();
			let firing_rate = status.param2 & 0b01111111;
			let shoot_stars = (status.param2 & 0b10000000) != 0;

			shot_bullet = rng.gen_range(0, 25) < (firing_rate + 2);
			if shot_bullet {
				let shoot_off_x;
				let shoot_off_y;

				if diff_y.abs() >= diff_x.abs() {
					// Shoot preferentially in the Y axis.
					shoot_off_x = 0;
					shoot_off_y = if diff_y > 0 { -1 } else { 1 };
				} else {
					shoot_off_x = if diff_x > 0 { -1 } else { 1 };
					shoot_off_y = 0;
				}

				let shoot_x = status.location_x as i16 + shoot_off_x;
				let shoot_y = status.location_y as i16 + shoot_off_y;
				sim.make_shoot_actions(shoot_x, shoot_y, shoot_off_x, shoot_off_y, shoot_stars, false, &mut actions);
			}
		} else {
			shot_bullet = false;
		}

		if !shot_bullet {
			let off_x;
			let off_y;

			// If param1 (intelligence) is 0, then it should always randomise, and when 8 it
			// should ALMOST always seek.
			let should_randomise: bool = rng.gen_range(0, 9) >= status.param1;
			if should_randomise {
				let (rand_step_x, rand_step_y) = sim.get_random_step();
				off_x = rand_step_x;
				off_y = rand_step_y;
			} else {
				let (seek_x, seek_y) = sim.seek_direction(status.location_x as i16, status.location_y as i16).to_offset();
				off_x = seek_x;
				off_y = seek_y;
			}

			let dest_x = status.location_x as i16 + off_x;
			let dest_y = status.location_y as i16 + off_y;
			if sim.has_player_at_location(dest_x, dest_y) {
				add_monster_touch_player_actions(status.location_x as i16, status.location_y as i16, &mut actions, sim);
			} else {
				let dest_behaviour = sim.behaviour_for_pos(dest_x, dest_y);
				if dest_behaviour.blocked(false) == BlockedStatus::NotBlocked {
					actions.push(Action::MoveTile {
						from_x: status.location_x as i16,
						from_y: status.location_y as i16,
						to_x: status.location_x as i16 + off_x,
						to_y: status.location_y as i16 + off_y,
						offset_x: off_x,
						offset_y: off_y,
						check_push: true,
						is_player: false,
					});
				}
			}
		}

		ActionResult::with_actions(actions)
	}

	fn push(&self, x: i16, y: i16, _push_off_x: i16, _push_off_y: i16, is_player: bool, sim: &BoardSimulator) -> PushResult {
		monster_push(x, y, is_player, sim)
	}

	fn damage(&self, x: i16, y: i16, damage_type: DamageType, sim: &BoardSimulator, actions: &mut Vec<Action>) -> DamageResult {
		monster_damage(self, x, y, damage_type, sim, actions)
	}

	fn destructable(&self) -> bool {
		true
	}
}
