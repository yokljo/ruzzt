use std::path::Path;
use zzt_file_format::World;

#[derive(Debug, PartialEq)]
enum FileType {
	Zzt,
	Json,
}

impl FileType {
	fn parse(type_str: &str) -> Result<FileType, String> {
		match type_str {
			"zzt" => Ok(FileType::Zzt),
			"json" => Ok(FileType::Json),
			_ => Err(type_str.into())
		}
	}
}

fn main() -> Result<(), String> {
	let matches = clap::App::new("zzt_to_json")
		.about("Converts between ZZT and JSON formats")
		.arg(clap::Arg::with_name("INPUT_TYPE")
			.help("The type of the input file: \"zzt\" or \"json\"")
			.required(true)
			.index(1))
		.arg(clap::Arg::with_name("OUTPUT_TYPE")
			.help("The type of the output file: \"zzt\" or \"json\"")
			.required(true)
			.index(2))
		.arg(clap::Arg::with_name("INPUT")
			.help("The input file")
			.required(true)
			.index(3))
		.get_matches();
	
	let input_type = FileType::parse(matches.value_of("INPUT_TYPE").unwrap())?;
	let output_type = FileType::parse(matches.value_of("OUTPUT_TYPE").unwrap())?;
	let input_file_path = Path::new(matches.value_of("INPUT").unwrap());
	let mut input_file = std::fs::File::open(input_file_path).map_err(|e| format!("{:?}", e))?;
	
	let loaded_world;
	
	eprintln!("Loading...");
	
	match input_type  {
		FileType::Zzt => {
			loaded_world = Some(World::parse(&mut input_file)?);
		}
		FileType::Json => {
			loaded_world = Some(serde_json::from_reader(input_file).map_err(|e| format!("{:?}", e))?);
		}
	}
	
	eprintln!("Saving...");
	if let Some(world) = loaded_world {
		match output_type {
			FileType::Json => {
				let json_str = serde_json::to_string_pretty(&world).map_err(|e| format!("{:?}", e))?;
				println!("{}", json_str);
			}
			FileType::Zzt => {
				world.write(&mut std::io::stdout()).map_err(|e| format!("Write failed: {:?}", e))?;
			}
		}
	}
	
	Ok(())
}
