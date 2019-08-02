use crate::behaviour::*;
use crate::board_message::*;
use crate::board_simulator::*;
use crate::event::*;
use crate::direction::*;

use zzt_file_format::*;

use crate::zzt_behaviours::monster_interactions::*;

#[derive(Debug, Clone)]
pub struct EmptyBehaviour;

impl Behaviour for EmptyBehaviour {
	fn push(&self, _x: i16, _y: i16, _push_off_x: i16, _push_off_y: i16, _is_player: bool, _sim: &BoardSimulator) -> PushResult {
		PushResult::do_nothing_not_blocked()
	}

	fn blocked(&self, _is_player: bool) -> BlockedStatus {
		BlockedStatus::NotBlocked
	}

	fn destructable(&self) -> bool {
		true
	}

	fn conveyable(&self) -> bool {
		true
	}
}

#[derive(Debug, Clone)]
pub struct BoardEdgeBehaviour;

impl Behaviour for BoardEdgeBehaviour {
	fn push(&self, _x: i16, _y: i16, push_off_x: i16, push_off_y: i16, is_player: bool, sim: &BoardSimulator) -> PushResult {
		if is_player {
			let direction = Direction::from_offset(push_off_x, push_off_y);
			let new_board_index = match direction {
				Direction::North => sim.board_meta_data.exit_north,
				Direction::South => sim.board_meta_data.exit_south,
				Direction::West => sim.board_meta_data.exit_west,
				Direction::East => sim.board_meta_data.exit_east,
				Direction::Idle => 0,
			} as usize;

			PushResult {
				blocked: BlockedStatus::Blocked,
				action_result: ActionResult::with_actions(vec![
					Action::SendBoardMessage(BoardMessage::SwitchBoard{new_board_index, direction}),
				]),
			}
		} else {
			PushResult::do_nothing_blocked()
		}
	}

	fn blocked(&self, _is_player: bool) -> BlockedStatus {
		BlockedStatus::Blocked
	}
}

/// This is what the player is replaced with in the title screen, so the title screen keys are
/// handled instead.
#[derive(Debug, Clone)]
pub struct MonitorBehaviour;

impl Behaviour for MonitorBehaviour {
	fn step(&self, event: Event, _status: &StatusElement, _status_index: usize, _sim: &BoardSimulator) -> ActionResult {
		let mut actions = vec![];

		match event {
			Event::PlayGame => actions.push(Action::SendBoardMessage(BoardMessage::PlayGame)),
			Event::Quit | Event::Escape => actions.push(Action::SendBoardMessage(BoardMessage::OpenQuitConfirmation)),
			Event::OpenWorldSelection => actions.push(Action::SendBoardMessage(BoardMessage::OpenWorldSelection)),
			Event::RestoreGame => actions.push(Action::SendBoardMessage(BoardMessage::OpenSaveSelection)),
			_ => {}
		}

		ActionResult::with_actions(actions)
	}
}

/*
param1 is 0 when shot by a player, 1 otherwise.

Bullets die when they run into each other.
*/
#[derive(Debug, Clone)]
pub struct BulletBehaviour;

impl Behaviour for BulletBehaviour {
	fn step(&self, _event: Event, status: &StatusElement, status_index: usize, sim: &BoardSimulator) -> ActionResult {
		let mut actions = vec![];

		if status.step_x != 0 || status.step_y != 0 {
			let mut new_step_x = status.step_x;
			let mut new_step_y = status.step_y;
			// First, check for ricochets.
			let next_x = status.location_x as i16 + new_step_x;
			let next_y = status.location_y as i16 + new_step_y;
			if let Some(dest_tile) = sim.get_tile(next_x, next_y) {
				if dest_tile.element_id == ElementType::Ricochet as u8 {
					// There is a ricochet in the path of the bullet.
					new_step_x *= -1;
					new_step_y *= -1;
				} else {
					let dest_behaviour = sim.behaviour_for_pos(next_x, next_y);
					if dest_behaviour.blocked_for_bullets() == BlockedStatus::Blocked {
						let cw_next_x = status.location_x as i16 + new_step_y;
						let cw_next_y = status.location_y as i16 + new_step_x;
						if let Some(cw_dest_tile) = sim.get_tile(cw_next_x, cw_next_y) {
							if cw_dest_tile.element_id == ElementType::Ricochet as u8 {
								// There is a ricochet clockwise to the step direction, so rotate
								// the step counter-clockwise.
								new_step_x = -status.step_y;
								new_step_y = -status.step_x;
							} else {
								let ccw_next_x = status.location_x as i16 - new_step_y;
								let ccw_next_y = status.location_y as i16 - new_step_x;
								if let Some(ccw_dest_tile) = sim.get_tile(ccw_next_x, ccw_next_y) {
									if ccw_dest_tile.element_id == ElementType::Ricochet as u8 {
										// There is a ricochet counter-clockwise to the step direction,
										// so rotate the step clockwise.
										new_step_x = status.step_y;
										new_step_y = status.step_x;
									}
								}
							}
						}
					}
				}
			}

			let mut bullet_died = false;

			let next_x = status.location_x as i16 + new_step_x;
			let next_y = status.location_y as i16 + new_step_y;

			let dest_behaviour = sim.behaviour_for_pos(next_x, next_y);
			if dest_behaviour.blocked_for_bullets() == BlockedStatus::Blocked {
				actions.push(Action::SetTile {
					x: status.location_x as i16,
					y: status.location_y as i16,
					tile: BoardTile {
						element_id: status.under_element_id,
						colour: status.under_colour,
					},
					status_element: None,
				});

				if dest_behaviour.destructable() {
					let by_player = status.param1 == 0;
					dest_behaviour.damage(next_x, next_y, DamageType::Shot{by_player}, sim, &mut actions);
				}

				// If there are a row of bullets shot going east: *****, and the rightmost bullet
				// hits something, and it didn't re-process the same status index, then the
				// rightmost bullet will die, the next one will be skipped, then the one after that
				// will run into the second rightmost bullet and die etc. making it so every second
				// bullet dies in one frame. By reprocessing the same index it will allow all the
				// bullets to keep moving after the rightmost one hits something.
				actions.push(Action::ReprocessSameStatusIndexOnRemoval);
				bullet_died = true;
			} else {
				if let Some(dest_tile) = sim.get_tile(next_x, next_y) {
					actions.push(Action::SetColour {
						x: status.location_x as i16,
						y: status.location_y as i16,
						colour: (if dest_tile.element_id == ElementType::Water as u8 { 0x70 } else { 0x00 }) | 0x0f,
					});
				}

				actions.push(Action::MoveTile {
					from_x: status.location_x as i16,
					from_y: status.location_y as i16,
					to_x: status.location_x as i16 + new_step_x,
					to_y: status.location_y as i16 + new_step_y,
					offset_x: new_step_x,
					offset_y: new_step_y,
					check_push: false,
					is_player: false,
				});
			}

			if !bullet_died && (new_step_x != status.step_x || new_step_y != status.step_y) {
				actions.push(Action::SetStep{
					status_index,
					step_x: new_step_x,
					step_y: new_step_y,
				});
			}
		}

		ActionResult {
			actions,
			continuation: None,
		}
	}

	fn push(&self, x: i16, y: i16, _push_off_x: i16, _push_off_y: i16, is_player: bool, sim: &BoardSimulator) -> PushResult {
		monster_push(x, y, is_player, sim)
	}

	fn destructable(&self) -> bool {
		true
	}
}

/*
Stars
param1 doesn't seem to do anything, but worth noting is that when you #throwstar, param1 is 1, and
when you #put dir star, param1 is 0.
param2 is the number of steps the star has left. This is 100 if you #throwstar, and 255 if you #put.

Both of these behaviours (with param1/2) can be explained by bullets and stars being fired by the
same function. Bullets shot by a player have param1 == 0, and param1 == 1 otherwise, so since stars
are never shot by a player, param1 will always be 1. If you fire a bullet, you will note that param2
is always 100, regardless of who shot it, which is because the bullet firing function sets it to 100
for the case where it is a star being fired (where param2 is the countdown).
*/
#[derive(Debug, Clone)]
pub struct StarBehaviour;

impl Behaviour for StarBehaviour {
	fn step(&self, _event: Event, status: &StatusElement, status_index: usize, sim: &BoardSimulator) -> ActionResult {
		let mut actions = vec![];

		if status.param2 == 0 {
			actions.push(Action::SetTile {
				x: status.location_x as i16,
				y: status.location_y as i16,
				tile: BoardTile { element_id: ElementType::Empty as u8, colour: 0 },
				status_element: None,
			});
		} else {
			let (seek_x, seek_y) = sim.seek_direction(status.location_x as i16, status.location_y as i16).to_offset();
			let dest_x = status.location_x as i16 + seek_x;
			let dest_y = status.location_y as i16 + seek_y;

			let (player_x, player_y) = sim.get_player_location();
			if player_x ==  dest_x && player_y == dest_y {
				add_monster_touch_player_actions(status.location_x as i16, status.location_y as i16, &mut actions, sim);
			} else {
				actions.push(Action::SetStep {
					status_index,
					step_x: seek_x,
					step_y: seek_y,
				});

				if status.param2 % 2 == 0 {
					let tile_opt = sim.get_status_tile(status);
					if let Some(tile) = tile_opt {
						let bg = tile.colour >> 4;
						let fg = tile.colour & 0b1111;
						let new_fg = ((fg - 8) % 7) + 9;
						let new_colour = (bg << 4) + new_fg;

						actions.push(Action::SetColour {
							x: status.location_x as i16,
							y: status.location_y as i16,
							colour: new_colour,
						});
					}

					let dest_behaviour = sim.behaviour_for_pos(dest_x, dest_y);
					if dest_behaviour.blocked_for_bullets() == BlockedStatus::NotBlocked {
						actions.push(Action::MoveTile {
							from_x: status.location_x as i16,
							from_y: status.location_y as i16,
							to_x: status.location_x as i16 + seek_x,
							to_y: status.location_y as i16 + seek_y,
							offset_x: seek_x,
							offset_y: seek_y,
							check_push: true,
							is_player: false,
						});
					}
				}
				actions.push(Action::SetStatusParam2{value: status.param2-1, status_index});
			}
		}

		ActionResult::with_actions(actions)
	}

	fn push(&self, x: i16, y: i16, _push_off_x: i16, _push_off_y: i16, is_player: bool, sim: &BoardSimulator) -> PushResult {
		monster_push(x, y, is_player, sim)
	}

	fn damage(&self, x: i16, y: i16, damage_type: DamageType, sim: &BoardSimulator, actions: &mut Vec<Action>) -> DamageResult {
		let has_status = sim.get_first_status_for_pos(x, y).is_some();
		if has_status && damage_type == DamageType::Bombed {
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
			default_damage_impl(self.destructable(), x, y, damage_type, sim, actions)
		}
	}
}
