use crate::board_simulator::*;

use zzt_file_format::*;

mod centipede;
mod creatures;
mod items;
mod misc;
mod monster_interactions;
mod terrains;

pub fn load_zzt_behaviours(sim: &mut BoardSimulator) {
	sim.set_behaviour(ElementType::Player, Box::new(items::PlayerBehaviour));
	sim.set_behaviour(ElementType::Ammo, Box::new(items::AmmoBehaviour));
	sim.set_behaviour(ElementType::Torch, Box::new(items::TorchBehaviour));
	sim.set_behaviour(ElementType::Gem, Box::new(items::GemBehaviour));
	sim.set_behaviour(ElementType::Key, Box::new(items::KeyBehaviour));
	sim.set_behaviour(ElementType::Door, Box::new(items::DoorBehaviour));
	sim.set_behaviour(ElementType::Scroll, Box::new(items::ScrollBehaviour));
	sim.set_behaviour(ElementType::Passage, Box::new(items::PassageBehaviour));
	sim.set_behaviour(ElementType::Duplicator, Box::new(items::DuplicatorBehaviour));
	sim.set_behaviour(ElementType::Bomb, Box::new(items::BombBehaviour));
	sim.set_behaviour(ElementType::Energizer, Box::new(items::EnergizerBehaviour));
	sim.set_behaviour(ElementType::Clockwise, Box::new(items::ConveyorBehaviour{clockwise: true}));
	sim.set_behaviour(ElementType::Counter, Box::new(items::ConveyorBehaviour{clockwise: false}));
	
	sim.set_behaviour(ElementType::Bear, Box::new(creatures::BearBehaviour));
	sim.set_behaviour(ElementType::Ruffian, Box::new(creatures::RuffianBehaviour));
	sim.set_behaviour(ElementType::Object, Box::new(creatures::ObjectBehaviour));
	sim.set_behaviour(ElementType::Slime, Box::new(creatures::SlimeBehaviour));
	sim.set_behaviour(ElementType::Shark, Box::new(creatures::SharkBehaviour));
	sim.set_behaviour(ElementType::SpinningGun, Box::new(creatures::SpinningGunBehaviour));
	sim.set_behaviour(ElementType::Pusher, Box::new(creatures::PusherBehaviour));
	sim.set_behaviour(ElementType::Lion, Box::new(creatures::LionBehaviour));
	sim.set_behaviour(ElementType::Tiger, Box::new(creatures::TigerBehaviour));
	
	sim.set_behaviour(ElementType::Head, Box::new(centipede::HeadBehaviour));
	sim.set_behaviour(ElementType::Segment, Box::new(centipede::SegmentBehaviour));
	
	sim.set_behaviour(ElementType::Water, Box::new(terrains::WaterBehaviour));
	sim.set_behaviour(ElementType::Forest, Box::new(terrains::ForestBehaviour));
	sim.set_behaviour(ElementType::Breakable, Box::new(terrains::BreakableBehaviour));
	sim.set_behaviour(ElementType::Boulder, Box::new(terrains::BoulderBehaviour));
	sim.set_behaviour(ElementType::SliderNS, Box::new(terrains::SliderNSBehaviour));
	sim.set_behaviour(ElementType::SliderEW, Box::new(terrains::SliderEWBehaviour));
	sim.set_behaviour(ElementType::Fake, Box::new(terrains::FakeBehaviour));
	sim.set_behaviour(ElementType::Invisible, Box::new(terrains::InvisibleBehaviour));
	sim.set_behaviour(ElementType::BlinkWall, Box::new(terrains::BlinkWallBehaviour));
	sim.set_behaviour(ElementType::Transporter, Box::new(terrains::TransporterBehaviour));
	sim.set_behaviour(ElementType::Ricochet, Box::new(terrains::RicochetBehaviour));
	
	sim.set_behaviour(ElementType::Empty, Box::new(misc::EmptyBehaviour));
	sim.set_behaviour(ElementType::BoardEdge, Box::new(misc::BoardEdgeBehaviour));
	sim.set_behaviour(ElementType::Monitor, Box::new(misc::MonitorBehaviour));
	sim.set_behaviour(ElementType::Bullet, Box::new(misc::BulletBehaviour));
	sim.set_behaviour(ElementType::Star, Box::new(misc::StarBehaviour));
}
