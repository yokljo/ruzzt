use crate::behaviour::*;
use crate::board_message::*;
use crate::board_simulator::*;
use crate::event::*;
use crate::sounds::*;

use num::FromPrimitive;

use zzt_file_format::*;

#[derive(Debug, Clone)]
pub struct WaterBehaviour;

impl Behaviour for WaterBehaviour {
	fn push(&self, _x: i16, _y: i16, _push_off_x: i16, _push_off_y: i16, is_player: bool, _sim: &BoardSimulator) -> PushResult {
		if is_player {
			PushResult {
				blocked: BlockedStatus::Blocked,
				action_result: ActionResult::with_actions(vec![Action::SendBoardMessage(BoardMessage::ShowOneTimeNotification(OneTimeNotification::BlockedByWater))]),
			}
		} else {
			PushResult::do_nothing_blocked()
		}
	}

	fn blocked_for_bullets(&self) -> BlockedStatus {
		BlockedStatus::NotBlocked
	}
}

#[derive(Debug, Clone)]
pub struct ForestBehaviour;

impl Behaviour for ForestBehaviour {
	fn push(&self, x: i16, y: i16, _push_off_x: i16, _push_off_y: i16, is_player: bool, _sim: &BoardSimulator) -> PushResult {
		if is_player {
			PushResult {
				blocked: BlockedStatus::NotBlocked,
				action_result: ActionResult::with_actions(vec![
					Action::SendBoardMessage(BoardMessage::PlaySoundArray(process_notes_string(b"ta"), SoundPriority::Level(3))),
					Action::SendBoardMessage(BoardMessage::ShowOneTimeNotification(OneTimeNotification::ForestCleared)),
					Action::SetTile {
						x,
						y,
						tile: BoardTile { element_id: ElementType::Empty as u8, colour: 0 },
						status_element: None,
					},
				]),
			}
		} else {
			PushResult::do_nothing_blocked()
		}
	}
}

#[derive(Debug, Clone)]
pub struct BreakableBehaviour;

impl Behaviour for BreakableBehaviour {
	fn destructable(&self) -> bool {
		true
	}
}

#[derive(Debug, Clone)]
pub struct BoulderBehaviour;

impl Behaviour for BoulderBehaviour {
	fn push(&self, x: i16, y: i16, push_off_x: i16, push_off_y: i16, _is_player: bool, _sim: &BoardSimulator) -> PushResult {
		PushResult {
			blocked: BlockedStatus::NotBlocked,
			action_result: ActionResult::with_actions(vec![
				Action::MoveTile{
					from_x: x,
					from_y: y,
					to_x: x + push_off_x,
					to_y: y + push_off_y,
					offset_x: push_off_x,
					offset_y: push_off_y,
					check_push: true,
					is_player: false,
				}
			]),
		}
	}

	fn conveyable(&self) -> bool {
		true
	}

	fn can_squash(&self) -> bool {
		true
	}
}

#[derive(Debug, Clone)]
pub struct SliderNSBehaviour;

impl Behaviour for SliderNSBehaviour {
	fn push(&self, x: i16, y: i16, push_off_x: i16, push_off_y: i16, _is_player: bool, _sim: &BoardSimulator) -> PushResult {
		if push_off_x == 0 {
			PushResult {
				blocked: BlockedStatus::NotBlocked,
				action_result: ActionResult::with_actions(vec![
					Action::MoveTile{
						from_x: x,
						from_y: y,
						to_x: x + push_off_x,
						to_y: y + push_off_y,
						offset_x: push_off_x,
						offset_y: push_off_y,
						check_push: true,
						is_player: false,
					}
				]),
			}
		} else {
			PushResult::do_nothing_blocked()
		}
	}

	fn can_squash(&self) -> bool {
		true
	}
}

#[derive(Debug, Clone)]
pub struct SliderEWBehaviour;

impl Behaviour for SliderEWBehaviour {
	fn push(&self, x: i16, y: i16, push_off_x: i16, push_off_y: i16, _is_player: bool, _sim: &BoardSimulator) -> PushResult {
		if push_off_y == 0 {
			PushResult {
				blocked: BlockedStatus::NotBlocked,
				action_result: ActionResult::with_actions(vec![
					Action::MoveTile{
						from_x: x,
						from_y: y,
						to_x: x + push_off_x,
						to_y: y + push_off_y,
						offset_x: push_off_x,
						offset_y: push_off_y,
						check_push: true,
						is_player: false,
					}
				]),
			}
		} else {
			PushResult::do_nothing_blocked()
		}
	}

	fn can_squash(&self) -> bool {
		true
	}
}

#[derive(Debug, Clone)]
pub struct FakeBehaviour;

impl Behaviour for FakeBehaviour {
	fn push(&self, _x: i16, _y: i16, _push_off_x: i16, _push_off_y: i16, _is_player: bool, _sim: &BoardSimulator) -> PushResult {
		PushResult::do_nothing_not_blocked()
	}

	fn blocked(&self, _is_player: bool) -> BlockedStatus {
		BlockedStatus::NotBlocked
	}

	fn destructable(&self) -> bool {
		true
	}
}

#[derive(Debug, Clone)]
pub struct InvisibleBehaviour;

impl Behaviour for InvisibleBehaviour {
	fn push(&self, x: i16, y: i16, _push_off_x: i16, _push_off_y: i16, is_player: bool, sim: &BoardSimulator) -> PushResult {
		if is_player {
			if let Some(tile) = sim.get_tile(x, y) {
				PushResult {
					blocked: BlockedStatus::Blocked,
					action_result: ActionResult::with_actions(vec![
						Action::SetTile {
							x,
							y,
							tile: BoardTile {
								element_id: ElementType::Normal as u8,
								colour: tile.colour,
							},
							status_element: None,
						},
					]),
				}
			} else {
				PushResult::do_nothing_blocked()
			}
		} else {
			PushResult::do_nothing_blocked()
		}
	}
}

#[derive(Debug, Clone)]
pub struct BlinkWallBehaviour;

impl Behaviour for BlinkWallBehaviour {
	fn step(&self, _event: Event, status: &StatusElement, status_index: usize, sim: &BoardSimulator) -> ActionResult {
		let wall_tile = sim.get_tile(status.location_x as i16, status.location_y as i16).unwrap();

		let mut actions = vec![];

		// If the period is 4 then the blink wall counts down from 9 to 1, then goes to 9 again.
		// every time it hits 1, it toggles the wall by checking the first tile adjacent to it to
		// see if the wall is there or not.

		let period = status.param2 + 1;
		let start_countdown = (period * 2) - 1;
		let mut countdown = status.param3;

		if countdown == 0 {
			countdown = status.param1;
		}

		let next_countdown = if countdown > 1 {
			countdown - 1
		} else {
			let mut is_adding_wall_opt = None;

			let mut current_x = status.location_x as i16;
			let mut current_y = status.location_y as i16;
			loop {
				current_x += status.step_x;
				current_y += status.step_y;
				let behaviour = sim.behaviour_for_pos(current_x, current_y);
				let current_tile = sim.get_tile(current_x, current_y).unwrap();

				let is_blink_ray = current_tile.element_id == ElementType::BlinkRayVertical as u8 || current_tile.element_id == ElementType::BlinkRayHorizontal as u8;
				let is_player = current_tile.element_id == ElementType::Player as u8;

				let is_adding_wall = *is_adding_wall_opt.get_or_insert(!is_blink_ray);
				if is_adding_wall {
					if !is_player && !behaviour.destructable() {
						break;
					}
				} else {
					if !is_blink_ray {
						break;
					}
				}

				let element_type = if is_adding_wall {
					if is_player {
						let mut offset_x = 0;
						let mut offset_y = 0;
						if status.step_x == 0 {
							offset_x = 1;
						} else {
							offset_y = -1;
						};

						// ZZT will end game if the space to both sides of the player relative to
						// the blink wall are NOT EMPTY (not just something that doesn't block, it
						// has to be the empty type)
						let adjacent_tile1 = sim.get_tile(current_x + offset_x, current_y + offset_y).unwrap();
						let adjacent_tile2 = sim.get_tile(current_x - offset_x, current_y - offset_y).unwrap();
						if adjacent_tile1.element_id != ElementType::Empty as u8 && adjacent_tile2.element_id != ElementType::Empty as u8 {
							// The player stops moving here, and the player's health visually counts
							// down by 10s until it reaches zero, but you cannot intercept it.
							// It should be fine to just end the game at this point.
							actions.push(Action::ModifyPlayerItem{
								item_type: PlayerItemType::Health,
								offset: -sim.world_header.player_health,
								require_exact_amount: false,
							});
							break;
						}

						actions.push(Action::MoveTile{
							from_x: current_x,
							from_y: current_y,
							to_x: current_x + offset_x,
							to_y: current_y + offset_y,
							offset_x,
							offset_y,
							// Blink walls will push the player on top of the thing next to it, no
							// matter what it is. This can cause the player to end up on top of a
							// wall or a pusher.
							check_push: false,
							is_player: false,
						});

						actions.push(Action::ModifyPlayerItem{
							item_type: PlayerItemType::Health,
							offset: -10,
							require_exact_amount: false,
						});
						actions.push(Action::CheckRestartOnZapped);
					}

					if status.step_x == 0 {
						ElementType::BlinkRayVertical
					} else {
						ElementType::BlinkRayHorizontal
					}
				} else {
					ElementType::Empty
				};

				actions.push(Action::SetTile {
					x: current_x,
					y: current_y,
					tile: BoardTile {
						element_id: element_type as u8,
						colour: wall_tile.colour,
					},
					status_element: None,
				});
			}

			start_countdown
		};


		actions.push(Action::SetStatusParam3{value: next_countdown, status_index});

		ActionResult::with_actions(actions)
	}
}

#[derive(Debug, Clone)]
pub struct TransporterContinuation {
	search_x: i16,
	search_y: i16,
	step_x: i16,
	step_y: i16,
}

// TODO: Bombs, boulders and sliders can be pushed through transporters by the player, stars,
//       duplicators, pushers, and some OOP commands (/dir, #go, #try, #put).
impl ActionContinuation for TransporterContinuation {
	fn next_step(&mut self, apply_action_report: ApplyActionResultReport, _status_index: usize, status: &StatusElement, sim: &BoardSimulator) -> ActionContinuationResult {
		// NOTE: In this function, status will be the currently processing status, which will be
		//       that of the player that pushed the transporter in the first place.
		let mut actions = vec![];

		let mut should_finish = true;

		if apply_action_report.move_was_blocked == BlockedStatus::Blocked {
			// Don't look for a transporter at the very edge of the board, because it would try to
			// move the player outside the board.
			while self.search_y >= 1 && self.search_y < BOARD_HEIGHT as i16 - 1
					&& self.search_x >= 1 && self.search_x < BOARD_WIDTH as i16 - 1
			{
				self.search_x += self.step_x;
				self.search_y += self.step_y;

				if let Some(search_tile) = sim.get_tile(self.search_x, self.search_y) {
					if ElementType::from_u8(search_tile.element_id) == Some(ElementType::Transporter) {
						if let Some((_, search_status)) = sim.get_first_status_for_pos(self.search_x, self.search_y) {
							if self.step_x == -search_status.step_x && self.step_y == -search_status.step_y {
								actions.push(Action::MoveTile {
									from_x: status.location_x as i16,
									from_y: status.location_y as i16,
									to_x: self.search_x + self.step_x,
									to_y: self.search_y + self.step_y,
									offset_x: self.step_x,
									offset_y: self.step_y,
									check_push: true,
									is_player: false,
								});

								// Keep searching in case this transporter is blocked, and it should
								// find the next transporter along if there is one.
								should_finish = false;

								break;
							}
						}
					}
				}
			}
		}

		ActionContinuationResult {
			actions,
			finished: should_finish,
		}
	}
}

#[derive(Debug, Clone)]
pub struct TransporterBehaviour;

impl Behaviour for TransporterBehaviour {
	fn step(&self, _event: Event, _status: &StatusElement, _status_index: usize, _sim: &BoardSimulator) -> ActionResult {
		ActionResult::do_nothing()
	}

	fn push(&self, x: i16, y: i16, push_off_x: i16, push_off_y: i16, is_player: bool, sim: &BoardSimulator) -> PushResult {
		if is_player {
			if let Some((_status_index, status)) = sim.get_first_status_for_pos(x, y) {
				if status.step_x == push_off_x && status.step_y == push_off_y {
					PushResult {
						blocked: BlockedStatus::Blocked,
						action_result: ActionResult {
							actions: vec![
								Action::SendBoardMessage(BoardMessage::PlaySoundArray(process_notes_string(b"sc+d-e+f#-g#+a#c+d"), SoundPriority::Level(3))),
								Action::MoveTile {
									from_x: x - status.step_x,
									from_y: y - status.step_y,
									to_x: x + status.step_x,
									to_y: y + status.step_y,
									offset_x: status.step_x,
									offset_y: status.step_y,
									check_push: true,
									is_player: false,
								}
							],
							continuation: Some(Box::new(TransporterContinuation {
								search_x: x,
								search_y: y,
								step_x: status.step_x,
								step_y: status.step_y,
							})),
						},
					}
				} else {
					PushResult::do_nothing_blocked()
				}
			} else {
				PushResult::do_nothing_blocked()
			}
		} else {
			PushResult::do_nothing_blocked()
		}
	}
}

#[derive(Debug, Clone)]
pub struct RicochetBehaviour;

impl Behaviour for RicochetBehaviour {
	// Ricochets don't really do anything. Bullets use them to redirect themselves.
}
