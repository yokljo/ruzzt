use crate::behaviour::*;
use crate::board_message::*;
use crate::board_simulator::*;
use crate::sounds::*;

use zzt_file_format::*;

// When a monster touches a player it dies and takes 10 health.
pub fn add_monster_touch_player_actions(x: i16, y: i16, actions: &mut Vec<Action>, sim: &BoardSimulator) {
	if let Some((_status_index, status)) = sim.get_first_status_for_pos(x, y) {
		actions.push(Action::SetTile {
			x,
			y,
			tile: BoardTile { element_id: status.under_element_id, colour: status.under_colour },
			status_element: None,
		});

		let (player_x, player_y) = sim.get_player_location();
		let behaviour = sim.behaviour_for_pos(player_x, player_y);
		behaviour.damage(player_x, player_y, DamageType::Other, sim, actions);

		if sim.world_header.energy_cycles <= 0 {
			actions.push(Action::ModifyPlayerItem {
				item_type: PlayerItemType::Health,
				offset: -10,
				require_exact_amount: false,
			});
			actions.push(Action::CheckRestartOnZapped);
		}
		// TODO: Play sound
	} else {
		// TODO: Do monsters hurt when they don't have a status?
	}
}

pub fn monster_push(x: i16, y: i16, is_player: bool, sim: &BoardSimulator) -> PushResult {
	let mut actions = vec![];
	let mut blocked = BlockedStatus::Blocked;

	if is_player {
		add_monster_touch_player_actions(x, y, &mut actions, sim);
		blocked = BlockedStatus::NotBlocked;
	}

	PushResult {
		blocked,
		action_result: ActionResult::with_actions(actions),
	}
}

pub fn monster_damage(behaviour: &dyn Behaviour, x: i16, y: i16, damage_type: DamageType, sim: &BoardSimulator, actions: &mut Vec<Action>) -> DamageResult {
	if let Some((_, ref status)) = sim.get_first_status_for_pos(x, y) {
		actions.push(Action::SetTile {
			x,
			y,
			tile: BoardTile { element_id: status.under_element_id, colour: status.under_colour },
			status_element: None,
		});

		actions.push(Action::SendBoardMessage(BoardMessage::PlaySoundArray(
			process_notes_string(b"c--c++++c--c"),
			SoundPriority::Level(3)
		)));

		DamageResult::Died
	} else {
		default_damage_impl(behaviour.destructable(), x, y, damage_type, sim, actions)
	}
}
