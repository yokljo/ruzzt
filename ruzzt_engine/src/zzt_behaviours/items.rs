use crate::behaviour::*;
use crate::board_message::*;
use crate::board_simulator::*;
use crate::event::*;
use crate::direction::*;
use crate::oop_parser::*;
use crate::sounds::*;

use rand::Rng;

use zzt_file_format::*;
use zzt_file_format::dosstring::DosString;

#[derive(Debug, Clone)]
pub struct PlayerBehaviour;

impl Behaviour for PlayerBehaviour {
	fn step(&self, event: Event, status: &StatusElement, status_index: usize, sim: &BoardSimulator) -> ActionResult {
		let mut actions = vec![];

		let is_end_of_game = sim.world_header.player_health <= 0;

		let mut move_direction_opt = None;

		match event {
			Event::Left | Event::Right | Event::Up | Event::Down => {
				if !is_end_of_game {
					move_direction_opt = match event {
						Event::Left => Some(Direction::West),
						Event::Right => Some(Direction::East),
						Event::Up => Some(Direction::North),
						Event::Down => Some(Direction::South),
						_ => None,
					};
				}
			}
			Event::ShootFlow | Event::ShootLeft | Event::ShootRight | Event::ShootUp | Event::ShootDown => {
				if !is_end_of_game {
					if sim.world_header.player_ammo > 0 {
						let (shoot_step_x, shoot_step_y) = match event {
							Event::ShootFlow => (status.step_x, status.step_y),
							Event::ShootLeft => (-1, 0),
							Event::ShootRight => (1, 0),
							Event::ShootUp => (0, -1),
							Event::ShootDown => (0, 1),
							_ => (0, 0),
						};

						let shoot_x = status.location_x as i16 + shoot_step_x;
						let shoot_y = status.location_y as i16 + shoot_step_y;

						let fired_shot = sim.make_shoot_actions(shoot_x, shoot_y, shoot_step_x, shoot_step_y, false, true, &mut actions);

						if fired_shot {
							actions.push(Action::ModifyPlayerItem {
								item_type: PlayerItemType::Ammo,
								offset: -1,
								require_exact_amount: false,
							});
							actions.push(Action::SendBoardMessage(BoardMessage::PlaySoundArray(
								process_notes_string(b"t+c-c-c"), SoundPriority::Level(2))));
						}
					} else {
						actions.push(Action::SendBoardMessage(BoardMessage::ShowOneTimeNotification(OneTimeNotification::NoAmmo)));
					}
				}
			}
			Event::LightTorch => {
				if let Some(torch_cycles) = sim.world_header.torch_cycles {
					if let Some(player_torches) = sim.world_header.player_torches {
						if torch_cycles == 0 {
							if player_torches > 0 {
								if sim.board_meta_data.is_dark {
									actions.push(Action::SetTorchCycles(200));
									actions.push(Action::ModifyPlayerItem {
										item_type: PlayerItemType::Torches,
										offset: -1,
										require_exact_amount: false,
									});
								} else {
									actions.push(Action::SendBoardMessage(BoardMessage::ShowOneTimeNotification(OneTimeNotification::LightTorchInLitRoom)));
								}
							} else {
								actions.push(Action::SendBoardMessage(BoardMessage::ShowOneTimeNotification(OneTimeNotification::NoTorches)));
							}
						}
					}
				}
			}
			Event::PauseGame => {
				if !is_end_of_game {
					actions.push(Action::SendBoardMessage(BoardMessage::PauseGame));
				}
			}
			Event::SaveGame => {
				actions.push(Action::SendBoardMessage(BoardMessage::OpenSaveGameInput));
			}
			Event::Debug => {
				actions.push(Action::SendBoardMessage(BoardMessage::OpenDebugInput));
			}
			Event::Quit | Event::Escape => {
				if is_end_of_game {
					actions.push(Action::SendBoardMessage(BoardMessage::ReturnToTitleScreen));
					let mut filename = sim.world_header.world_name.clone();
					filename = filename.to_upper();
					filename += b".ZZT";
					actions.push(Action::SendBoardMessage(BoardMessage::OpenWorld{filename}));
				} else {
					actions.push(Action::SendBoardMessage(BoardMessage::OpenEndGameConfirmation));
				}
			}
			_ => {}
		};

		actions.push(Action::SetAsPlayerTile {
			x: status.location_x as i16,
			y: status.location_y as i16,
		});

		if let Some(move_direction) = move_direction_opt {
			let (off_x, off_y) = move_direction.to_offset();

			actions.push(Action::SetStep {
				status_index,
				step_x: off_x,
				step_y: off_y,
			});

			let to_x = status.location_x as i16 + off_x;
			let to_y = status.location_y as i16 + off_y;

			// ZZT always moves status element 0, resulting in some very weird behaviours when there
			// are multiple players.
			let status_to_move = &sim.status_elements[0];
			let from_x = status_to_move.location_x as i16;
			let from_y = status_to_move.location_y as i16;

			actions.push(Action::MoveTile {
				from_x,
				from_y,
				to_x,
				to_y,
				offset_x: off_x,
				offset_y: off_y,
				check_push: true,
				is_player: true,
			});
		}

		// Yes, if there are multiple players, torch and energy cycles go down faster.
		if let Some(torch_cycles) = sim.world_header.torch_cycles {
			if torch_cycles > 0 {
				actions.push(Action::SetTorchCycles(torch_cycles - 1));
			}
		}

		if sim.world_header.energy_cycles > 0 {
			if sim.world_header.energy_cycles == 10 {
				actions.push(Action::SendBoardMessage(BoardMessage::PlaySoundArray(process_notes_string(b"s.-c-a#gf#fd#c"), SoundPriority::Level(9))));
			}
			actions.push(Action::SetEnergyCycles(sim.world_header.energy_cycles - 1));
		}

		if sim.board_meta_data.time_limit > 0 {
			actions.push(Action::CheckTimeElapsed);
		}

		ActionResult::with_actions(actions)
	}

	fn push(&self, x: i16, y: i16, push_off_x: i16, push_off_y: i16, is_player: bool, _sim: &BoardSimulator) -> PushResult {
		if is_player {
			PushResult::do_nothing_not_blocked()
		} else {
			PushResult {
				blocked: BlockedStatus::NotBlocked,
				action_result: ActionResult::with_actions(vec![Action::MoveTile {
						from_x: x,
						from_y: y,
						to_x: x + push_off_x,
						to_y: y + push_off_y,
						offset_x: push_off_x,
						offset_y: push_off_y,
						check_push: true,
						is_player: false,
					}]),
			}
		}
	}

	fn destructable(&self) -> bool {
		true
	}

	fn conveyable(&self) -> bool {
		true
	}

	fn blocked(&self, is_player: bool) -> BlockedStatus {
		if is_player {
			// Player is not blocked for player so that when a player tries to walk from one board
			// to another and the player for the destination board is in the way, the player from
			// the origin board can still walk there.
			BlockedStatus::NotBlocked
		} else {
			BlockedStatus::Blocked
		}
	}

	fn damage(&self, _x: i16, _y: i16, _damage_type: DamageType, sim: &BoardSimulator, actions: &mut Vec<Action>) -> DamageResult {
		// NOTE: Players can shoot themselves. Proof is that when one bounces off a ricochet, it
		// comes back and hurts the player.
		if sim.world_header.energy_cycles <= 0 {
			actions.push(Action::SendBoardMessage(BoardMessage::PlaySoundArray(process_notes_string(b"--c+c-d#+d#"), SoundPriority::Level(2))));
			actions.push(Action::SendBoardMessage(BoardMessage::OpenScroll{title: DosString::new(), content_lines: vec![DosString::from_slice(b"Ouch!")]}));
			actions.push(Action::ModifyPlayerItem {
				item_type: PlayerItemType::Health,
				offset: -10,
				require_exact_amount: false,
			});
			actions.push(Action::CheckRestartOnZapped);
		}
		DamageResult::None
	}
}

#[derive(Debug, Clone)]
pub struct AmmoBehaviour;

impl Behaviour for AmmoBehaviour {
	fn push(&self, x: i16, y: i16, push_off_x: i16, push_off_y: i16, is_player: bool, _sim: &BoardSimulator) -> PushResult {
		if is_player {
			PushResult {
				blocked: BlockedStatus::NotBlocked,
				action_result: ActionResult::with_actions(vec![
					Action::SendBoardMessage(BoardMessage::PlaySoundArray(process_notes_string(b"cc#d"), SoundPriority::Level(2))),
					Action::SetTile {
						x,
						y,
						tile: BoardTile { element_id: ElementType::Empty as u8, colour: 0 },
						status_element: None,
					},
					Action::ModifyPlayerItem {
						item_type: PlayerItemType::Ammo,
						offset: 5,
						require_exact_amount: false,
					},
				]),
			}
		} else {
			PushResult {
				blocked: BlockedStatus::NotBlocked,
				action_result: ActionResult::with_actions(vec![Action::MoveTile {
					from_x: x,
					from_y: y,
					to_x: x + push_off_x,
					to_y: y + push_off_y,
					offset_x: push_off_x,
					offset_y: push_off_y,
					check_push: true,
					is_player: false,
				}]),
			}
		}
	}

	fn conveyable(&self) -> bool {
		true
	}
}

#[derive(Debug, Clone)]
pub struct TorchBehaviour;

impl Behaviour for TorchBehaviour {
	fn push(&self, x: i16, y: i16, _push_off_x: i16, _push_off_y: i16, is_player: bool, _sim: &BoardSimulator) -> PushResult {
		if is_player {
			PushResult {
				blocked: BlockedStatus::NotBlocked,
				action_result: ActionResult::with_actions(vec![
					Action::SendBoardMessage(BoardMessage::PlaySoundArray(process_notes_string(b"case"), SoundPriority::Level(3))),
					Action::SetTile {
						x,
						y,
						tile: BoardTile { element_id: ElementType::Empty as u8, colour: 0 },
						status_element: None,
					},
					Action::ModifyPlayerItem {
						item_type: PlayerItemType::Torches,
						offset: 1,
						require_exact_amount: false,
					},
				]),
			}
		} else {
			PushResult::do_nothing_blocked()
		}
	}
}

#[derive(Debug, Clone)]
pub struct GemBehaviour;

impl Behaviour for GemBehaviour {
	fn push(&self, x: i16, y: i16, push_off_x: i16, push_off_y: i16, is_player: bool, _sim: &BoardSimulator) -> PushResult {
		if is_player {
			PushResult {
				blocked: BlockedStatus::NotBlocked,
				action_result: ActionResult::with_actions(vec![
					Action::SendBoardMessage(BoardMessage::PlaySoundArray(process_notes_string(b"+c-gec"), SoundPriority::Level(2))),
					Action::SetTile {
						x,
						y,
						tile: BoardTile { element_id: ElementType::Empty as u8, colour: 0 },
						status_element: None,
					},
					Action::ModifyPlayerItem {
						item_type: PlayerItemType::Gems,
						offset: 1,
						require_exact_amount: false,
					},
					Action::ModifyPlayerItem {
						item_type: PlayerItemType::Score,
						offset: 10,
						require_exact_amount: false,
					},
					Action::ModifyPlayerItem {
						item_type: PlayerItemType::Health,
						offset: 1,
						require_exact_amount: false,
					},
				]),
			}
		} else {
			PushResult {
				blocked: BlockedStatus::NotBlocked,
				action_result: ActionResult::with_actions(vec![
					Action::MoveTile {
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
	}

	fn blocked(&self, _is_player: bool) -> BlockedStatus {
		BlockedStatus::Blocked
	}

	fn destructable(&self) -> bool {
		true
	}

	fn conveyable(&self) -> bool {
		true
	}

	fn can_be_squashed(&self) -> bool {
		true
	}

	fn damage(&self, x: i16, y: i16, damage_type: DamageType, sim: &BoardSimulator, actions: &mut Vec<Action>) -> DamageResult {
		if damage_type == (DamageType::Shot{by_player: false}) {
			// Although gems are destructable, they will only die when shot by a player.
			DamageResult::None
		} else {
			default_damage_impl(self.destructable(), x, y, damage_type, sim, actions)
		}
	}
}

#[derive(Debug, Clone)]
pub struct KeyBehaviour;

fn get_key_name(index: u8) -> &'static [u8] {
	match index {
		0 => b"Blue",
		1 => b"Green",
		2 => b"Cyan",
		3 => b"Red",
		4 => b"Purple",
		5 => b"Yellow",
		6 => b"White",
		_ => b"?",
	}
}

impl Behaviour for KeyBehaviour {
	fn push(&self, x: i16, y: i16, push_off_x: i16, push_off_y: i16, is_player: bool, sim: &BoardSimulator) -> PushResult {
		if is_player {
			if let Some(tile) = sim.get_tile(x, y) {
				let key_index = tile.colour as isize - 9;
				println!("{:?}", tile);
				if key_index >= 0 && key_index < 7 {
					let current_has_key = sim.world_header.player_keys[key_index as usize];
					if !current_has_key {
						let mut message_str = DosString::new();
						message_str += b"You now have the ";
						message_str += get_key_name(key_index as u8);
						message_str += b" key";

						PushResult {
							blocked: BlockedStatus::NotBlocked,
							action_result: ActionResult::with_actions(vec![
								Action::SendBoardMessage(BoardMessage::PlaySoundArray(process_notes_string(b"t+cegcegcegs+c"), SoundPriority::Level(2))),
								Action::SendBoardMessage(BoardMessage::OpenScroll {
									title: DosString::new(),
									content_lines: vec![message_str],
								}),
								Action::SetTile {
									x,
									y,
									tile: BoardTile { element_id: ElementType::Empty as u8, colour: 0 },
									status_element: None,
								},
								Action::ModifyPlayerKeys {
									index: key_index as u8,
									value: true,
								},
							]),
						}
					} else {
						let mut message_str = DosString::new();
						message_str += b"You already have the ";
						message_str += get_key_name(key_index as u8);
						message_str += b" key!";

						// TODO: Play sound
						PushResult {
							blocked: BlockedStatus::Blocked,
							action_result: ActionResult::with_actions(vec![
								Action::SendBoardMessage(BoardMessage::OpenScroll {
									title: DosString::new(),
									content_lines: vec![message_str],
								}),
							]),
						}
					}
				} else {
					PushResult::do_nothing_blocked()
				}
			} else {
				PushResult::do_nothing_blocked()
			}
		} else {
			PushResult {
				blocked: BlockedStatus::NotBlocked,
				action_result: ActionResult::with_actions(vec![Action::MoveTile {
					from_x: x,
					from_y: y,
					to_x: x + push_off_x,
					to_y: y + push_off_y,
					offset_x: push_off_x,
					offset_y: push_off_y,
					check_push: true,
					is_player: false,
				}]),
			}
		}
	}

	fn conveyable(&self) -> bool {
		true
	}
}

#[derive(Debug, Clone)]
pub struct DoorBehaviour;

impl Behaviour for DoorBehaviour {
	fn push(&self, x: i16, y: i16, _push_off_x: i16, _push_off_y: i16, is_player: bool, sim: &BoardSimulator) -> PushResult {
		let mut actions = vec![];

		if is_player {
			if let Some(tile) = sim.get_tile(x, y) {
				let key_index = ((tile.colour & 0xf0) >> 4) as isize - 1;
				if key_index >= 0 && key_index < 7 {
					let has_key = sim.world_header.player_keys[key_index as usize];

					if has_key {
						actions.push(Action::SetTile {
							x,
							y,
							tile: BoardTile { element_id: ElementType::Empty as u8, colour: 0 },
							status_element: None,
						});
						actions.push(Action::ModifyPlayerKeys {
							index: key_index as u8,
							value: false,
						});
						actions.push(Action::SendBoardMessage(BoardMessage::PlaySoundArray(process_notes_string(b"tcgbcgbi+c"), SoundPriority::Level(3))));

						let mut message_str = DosString::new();
						message_str += b"The ";
						message_str += get_key_name(key_index as u8);
						message_str += b" door is now open.";
						actions.push(Action::SendBoardMessage(BoardMessage::OpenScroll {
							title: DosString::new(),
							content_lines: vec![message_str],
						}));
					} else {
						actions.push(Action::SendBoardMessage(BoardMessage::PlaySoundArray(process_notes_string(b"--tgc"), SoundPriority::Level(3))));

						let mut message_str = DosString::new();
						message_str += b"The ";
						message_str += get_key_name(key_index as u8);
						message_str += b" door is locked!";
						actions.push(Action::SendBoardMessage(BoardMessage::OpenScroll {
							title: DosString::new(),
							content_lines: vec![message_str],
						}));
					}
				}
			}
		}

		PushResult {
			blocked: BlockedStatus::NotBlocked,
			action_result: ActionResult::with_actions(actions),
		}
	}
}

#[derive(Debug, Clone)]
pub struct ScrollBehaviour;

impl Behaviour for ScrollBehaviour {
	fn step(&self, _event: Event, status: &StatusElement, _status_index: usize, sim: &BoardSimulator) -> ActionResult {
		let tile_opt = sim.get_status_tile(status);
		if let Some(tile) = tile_opt {
			let bg = tile.colour >> 4;
			let fg = tile.colour & 0b1111;
			let new_fg = ((fg - 8) % 7) + 9;
			let new_colour = (bg << 4) + new_fg;

			ActionResult::with_actions(vec![
				Action::SetColour {
					x: status.location_x as i16,
					y: status.location_y as i16,
					colour: new_colour,
				}
			])
		} else {
			ActionResult::do_nothing()
		}
	}

	fn push(&self, x: i16, y: i16, push_off_x: i16, push_off_y: i16, is_player: bool, sim: &BoardSimulator) -> PushResult {
		let mut actions = vec![];
		let mut continuation: Option<Box<ActionContinuation>> = None;

		if is_player {
			let status_element_opt = sim.get_first_status_for_pos(x, y);

			if let Some((status_index, _status)) = status_element_opt {
				continuation = Some(Box::new(OopExecutionState::new(true, Some(status_index))));

				actions.push(Action::SendBoardMessage(BoardMessage::PlaySoundArray(process_notes_string(b"tc-c+d-d+e-e+f-f+g-g"), SoundPriority::Level(2))));
			}
		} else {
			actions.push(Action::MoveTile{
				from_x: x,
				from_y: y,
				to_x: x + push_off_x,
				to_y: y + push_off_y,
				offset_x: push_off_x,
				offset_y: push_off_y,
				check_push: true,
				is_player: false,
			});
		}

		PushResult {
			blocked: BlockedStatus::Blocked,
			action_result: ActionResult {
				actions,
				continuation,
			},
		}
	}

	fn conveyable(&self) -> bool {
		true
	}
}

#[derive(Debug, Clone)]
pub struct PassageBehaviour;

impl Behaviour for PassageBehaviour {
	fn push(&self, x: i16, y: i16, _push_off_x: i16, _push_off_y: i16, is_player: bool, sim: &BoardSimulator) -> PushResult {
		// To find the destination passage of a given colour on a board, start at the bottom right
		// and move up, then left when it reaches the top (weird...).
		// Passages block the way of the player, so the player doesn't move on the source board when
		// it teleports. When moving the player from the original location on the board to the
		// destination passage's tile, it always replaces the old tile with empty, not with the
		// under element id/colour. On the destination board, the player is played directly on top
		// of the passage tile, and the under element id/colour is set to the passage it is on top
		// of. It then pauses the game, making the player blink, which reveals that when the player
		// disappears during the pause blink cycle, the passage underneath is displayed, not just
		// empty like it looks like most of the time. This only happens for passages, so the pause
		// blink when the player is on top of a fake wall is to a black tile (empty).
		// param3 is the destination board index. Maybe once upon a time param1 and param2 were used
		// for the destination X/Y position, but the author changed their mind.
		// cycle is usually 0 because they don't do anything on step.
		if is_player {
			let status_element_opt = sim.get_first_status_for_pos(x, y);

			if let Some((_, status_element)) = status_element_opt {
				let tile_opt = sim.get_tile(x, y);
				let colour = if let Some(tile) = tile_opt {
					tile.colour
				} else {
					0
				};

				PushResult {
					blocked: BlockedStatus::Blocked,
					action_result: ActionResult::with_actions(vec![
						Action::SendBoardMessage(BoardMessage::PlaySoundArray(
							process_notes_string(b"tcegc#fg#df#ad#ga#eg#+c"), SoundPriority::Level(4))),
						Action::SendBoardMessage(BoardMessage::TeleportToBoard {
							destination_board_index: status_element.param3,
							passage_colour: colour,
						}),
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
struct DuplicatorContinuation;

impl ActionContinuation for DuplicatorContinuation {
	fn next_step(&mut self, apply_action_report: ApplyActionResultReport, _status_index: usize, status: &StatusElement, sim: &BoardSimulator) -> ActionContinuationResult {
		let mut actions = vec![];

		if apply_action_report.move_was_blocked == BlockedStatus::NotBlocked {
			// Duplicate!
			let source_x = status.location_x as i16 + status.step_x;
			let source_y = status.location_y as i16 + status.step_y;

			if let Some(source_tile) = sim.get_tile(source_x, source_y) {
				let dest_x = status.location_x as i16 - status.step_x;
				let dest_y = status.location_y as i16 - status.step_y;

				let mut duplicated_status_opt = sim.get_first_status_for_pos(source_x, source_y).map(|(_, status)| status.clone());
				if let Some(ref mut duplicated_status) = duplicated_status_opt {
					duplicated_status.location_x = dest_x as u8;
					duplicated_status.location_y = dest_y as u8;
				}

				actions.push(Action::SetTile {
					x: dest_x,
					y: dest_y,
					tile: source_tile,
					status_element: duplicated_status_opt,
				});

				actions.push(Action::SendBoardMessage(BoardMessage::PlaySoundArray(process_notes_string(b"scdefg"), SoundPriority::Level(3))));
			}
		} else {
			actions.push(Action::SendBoardMessage(BoardMessage::PlaySoundArray(process_notes_string(b"--g#f#"), SoundPriority::Level(3))));
		}

		ActionContinuationResult {
			actions,
			finished: true,
		}
	}
}

#[derive(Debug, Clone)]
pub struct DuplicatorBehaviour;

impl Behaviour for DuplicatorBehaviour {
	fn step(&self, _event: Event, status: &StatusElement, status_index: usize, sim: &BoardSimulator) -> ActionResult {
		// param1 is the progress (0-4)
		// param2 is the progression speed (0-8). This value is used to set the cycle every step.
		// The step x/y determines the source offset.

		let next_step = (status.param1 + 1) % 5;

		let mut actions = vec![Action::SetStatusParam1{value: next_step, status_index}];
		let mut continuation: Option<Box<ActionContinuation>> = None;

		if next_step == 0 {
			let source_x = status.location_x as i16 + status.step_x;
			let source_y = status.location_y as i16 + status.step_y;

			let (player_x, player_y) = sim.get_player_location();

			if source_x != player_x || source_y != player_y {
				if let Some(source_tile) = sim.get_tile(source_x, source_y) {
					// Yes, duplicators can duplicate board edges.
					if source_tile.element_id != ElementType::Empty as u8 {
						let dest_x = status.location_x as i16 - status.step_x;
						let dest_y = status.location_y as i16 - status.step_y;

						actions.push(Action::PushTile {
							x: dest_x,
							y: dest_y,
							offset_x: -status.step_x,
							offset_y: -status.step_y,
						});

						continuation = Some(Box::new(DuplicatorContinuation));
					}
				}
			}
		}

		// ZZT achieves a slower duplication by setting the cycle based on the value of param2.
		let expected_cycle = (9 - status.param2 as i16) * 3;

		if status.cycle != expected_cycle {
			actions.push(Action::SetCycle{status_index, cycle: expected_cycle});
		}

		ActionResult {
			actions,
			continuation,
		}
	}
}

// TODO: Bombs should not destroy the player.
// param1 is 0 when the bomb is doing nothing, or > 0 to represent the current count-down value.
#[derive(Debug, Clone)]
pub struct BombBehaviour;

// Bombable types: gem, bear, ruffian, lion, tiger, head, segment, breakable
impl Behaviour for BombBehaviour {
	fn push(&self, x: i16, y: i16, push_off_x: i16, push_off_y: i16, is_player: bool, sim: &BoardSimulator) -> PushResult {
		let mut actions = vec![];
		let mut blocked = BlockedStatus::Blocked;
		let mut just_triggered = false;

		if is_player {
			if let Some((_, status)) = sim.get_first_status_for_pos(x, y) {
				if status.param1 == 0 {
					just_triggered = true;
				}
			}
		}

		if just_triggered {
			if let Some((status_index, _)) = sim.get_first_status_for_pos(x, y) {
				actions.push(Action::SetStatusParam1{value: 9, status_index});
			}
		} else {
			actions.push(Action::MoveTile{
				from_x: x,
				from_y: y,
				to_x: x + push_off_x,
				to_y: y + push_off_y,
				offset_x: push_off_x,
				offset_y: push_off_y,
				check_push: true,
				is_player: false,
			});
			blocked = BlockedStatus::NotBlocked;
		}

		PushResult {
			blocked,
			action_result: ActionResult::with_actions(actions),
		}
	}

	fn step(&self, _event: Event, status: &StatusElement, status_index: usize, sim: &BoardSimulator) -> ActionResult {
		let mut actions = vec![];
		if status.param1 > 0 {
			if status.param1 <= 2 {
				// Blow up!
				let circle_height = CIRCLE_MASK.len() as i16;
				let mut y = status.location_y as i16 - ((circle_height - 1) / 2);

				for row in &CIRCLE_MASK {
					let mut x = status.location_x as i16 - ((CIRCLE_MASK_WIDTH as i16 - 1) / 2);
					let mut row_bits = *row;
					for _col_index in 0 .. CIRCLE_MASK_WIDTH {
						if row_bits & 0b1 == 1 && x >= 0 && x < BOARD_WIDTH as i16 && y >= 0 && y < BOARD_HEIGHT as i16 {
							let behaviour = sim.behaviour_for_pos(x, y);

							if let Some(tile) = sim.get_tile(x, y) {
								match status.param1 {
									1 => {
										if tile.element_id == ElementType::Breakable as u8 {
											// Clean up explosion.
											actions.push(Action::SetTile {
												x,
												y,
												tile: BoardTile {
													element_id: ElementType::Empty as u8,
													colour: 0,
												},
												status_element: None,
											});
										}
									}
									2 => {
										// Blow up.
										let damage_result = behaviour.damage(x, y, DamageType::Bombed, sim, &mut actions);

										if damage_result == DamageResult::Died {
											let mut rng = rand::thread_rng();
											let rand_colour: u8 = rng.gen_range(9, 16);
											actions.push(Action::SetTile {
												x,
												y,
												tile: BoardTile {
													element_id: ElementType::Breakable as u8,
													colour: rand_colour,
												},
												status_element: None,
											});
										}
									}
									_ => {}
								}
							}
						}
						row_bits >>= 1;
						x += 1;
					}
					y += 1;
				}
			}

			actions.push(Action::SetStatusParam1{value: status.param1 - 1, status_index});

			if status.param1 == 1 {
				actions.push(Action::SetTile {
					x: status.location_x as i16,
					y: status.location_y as i16,
					tile: BoardTile {
						element_id: ElementType::Empty as u8,
						colour: 0,
					},
					status_element: None,
				});
			}
		}
		ActionResult::with_actions(actions)
	}

	fn conveyable(&self) -> bool {
		true
	}

	fn can_squash(&self) -> bool {
		true
	}
}

#[derive(Debug, Clone)]
pub struct EnergizerBehaviour;

impl Behaviour for EnergizerBehaviour {
	fn push(&self, x: i16, y: i16, _push_off_x: i16, _push_off_y: i16, is_player: bool, _sim: &BoardSimulator) -> PushResult {
		// The energizer blinks for 75 cycles.
		if is_player {
			PushResult {
				blocked: BlockedStatus::NotBlocked,
				action_result: ActionResult::with_actions(vec![
					Action::SetEnergyCycles(75),
					Action::SendBoardMessage(BoardMessage::PlaySoundArray(
						process_notes_string(b"s.-cd#ef+f-fd#c+c-d#ef+f-fd#c+c-d#ef+f-fd#c+c-d#ef+f-fd#c+c-d#ef+f-fd#c+c-d#ef+f-fd#c+c-d#ef+f-fd#c"),
						SoundPriority::Level(9)
					)),
					Action::SendBoardMessage(BoardMessage::ShowOneTimeNotification(OneTimeNotification::PickUpEnergizer)),
					Action::SetTile {
						x,
						y,
						tile: BoardTile { element_id: ElementType::Empty as u8, colour: 0 },
						status_element: None,
					},
					Action::OthersApplyLabelOperation {
						current_status_index: None,
						receiver_name_opt: None,
						label: DosString::from_slice(b"energize"),
						operation: LabelOperation::Jump,
					}
				]),
			}
		} else {
			PushResult::do_nothing_blocked()
		}
	}

	fn conveyable(&self) -> bool {
		true
	}
}

#[derive(Debug, Clone)]
pub struct ConveyorBehaviour {
	pub clockwise: bool,
}

impl Behaviour for ConveyorBehaviour {
	fn step(&self, _event: Event, status: &StatusElement, _status_index: usize, sim: &BoardSimulator) -> ActionResult {
		let mut actions = vec![];

		let centre_x = status.location_x as i16;
		let centre_y = status.location_y as i16;
		// This is in clockwise order:
		let offsets: [(i16, i16); 8] = [(-1, -1), (0, -1), (1, -1), (1, 0), (1, 1), (0, 1), (-1, 1), (-1, 0)];

		#[derive(Debug)]
		struct RotatingEntry {
			tile: BoardTile,
			status_index: Option<usize>,
			fixed: bool,
			last_fixed: bool,
		}

		let mut rotating_tiles: Vec<RotatingEntry> = vec![];
		for offset in offsets.iter() {
			let pos_x = centre_x + offset.0;
			let pos_y = centre_y + offset.1;
			if let Some(tile) = sim.get_tile(pos_x, pos_y) {
				let status_index_opt = sim.get_first_status_for_pos(pos_x, pos_y).map(|(status_index, _)| status_index);
				let behaviour = sim.behaviour_for_pos(pos_x, pos_y);

				rotating_tiles.push(RotatingEntry {
					tile,
					status_index: status_index_opt,
					fixed: !behaviour.conveyable(),
					last_fixed: false,
				});
			} else {
				rotating_tiles.push(RotatingEntry {
					tile: BoardTile {
						element_id: 0,
						colour: 0,
					},
					status_index: None,
					fixed: true,
					last_fixed: false,
				});
			}
		}

		// Propagate the fixed values to adjacent tiles.
		let mut last_fixed = false;
		let mut process_fixed = |rotating_tile: &mut RotatingEntry| {
			rotating_tile.last_fixed = last_fixed;

			if rotating_tile.tile.element_id == ElementType::Empty as u8 {
				last_fixed = false;
			} else {
				if last_fixed {
					rotating_tile.fixed = true;
				}

				last_fixed = rotating_tile.fixed;
			}
		};

		if self.clockwise {
			for _ in 0..2 {
				for rotating_tile in rotating_tiles.iter_mut().rev() {
					process_fixed(rotating_tile);
				}
			}
			rotating_tiles.rotate_right(1);
		} else {
			for _ in 0..2 {
				for rotating_tile in rotating_tiles.iter_mut() {
					process_fixed(rotating_tile);
				}
			}
			rotating_tiles.rotate_left(1);
		}

		for (offset, tile_desc) in offsets.into_iter().zip(rotating_tiles.into_iter()) {
			let x = centre_x + offset.0;
			let y = centre_y + offset.1;

			if !tile_desc.fixed {
				if !tile_desc.last_fixed {
					actions.push(Action::SetTileElementIdAndColour {
						x,
						y,
						element_id: Some(tile_desc.tile.element_id),
						colour: Some(tile_desc.tile.colour),
					});

					if let Some(status_index) = tile_desc.status_index {
						actions.push(Action::SetStatusLocation {
							x,
							y,
							status_index,
						});
					}
				}
			} else if !tile_desc.last_fixed {
				actions.push(Action::SetTileElementIdAndColour {
					x,
					y,
					element_id: Some(ElementType::Empty as u8),
					colour: Some(15),
				});
			}
		}

		ActionResult::with_actions(actions)
	}
}
