import { zzt_to_json, zzt_colour_to_rgb, WorldState, default as init } from "/pkg/zzt_web_editor.js";
async function run() {
	await init("/pkg/zzt_web_editor_bg.wasm");
	//const result = zzt_to_json("asdf");
	//console.log(result);
	window.main = new Main()
}
run()

function getBG(zzt_col) {
	let bg_col = (zzt_col & 0xF0) >> 8
	
}

class Main {
	constructor() {
		this.board_canvas = document.getElementById("board_canvas")
		this.board_cxt = this.board_canvas.getContext("2d")
		this.font = new Image()
		this.font.src = "dosfont.png"
		
		this.status_elements_div = document.getElementById("status_elements")
		this.boards_list = document.getElementById("boards_list")
		
		this.current_board_index = 0
	}
	
	load_zzt_file(files) {
		let file = files[0]
		var reader = new FileReader()

		// Closure to capture the file information.
		reader.onload = () => {
			let data = reader.result
			let u8data = new Uint8Array(data)
			this.world_state = WorldState.from_file_data(u8data)
			//this.world = JSON.parse(this.world_state.get_world_json())
			//console.log(this.world)
			this.status_elements = JSON.parse(this.world_state.get_status_elements_json(this.world_state.get_current_board_index()))
			this.board_meta_data = JSON.parse(this.world_state.get_status_elements_json(this.world_state.get_current_board_index()))
			this.populate_status_editors()
			this.populate_boards_list()
			console.log(this.status_elements)
			this.render()
		}

		reader.readAsArrayBuffer(file)
	}
	
	create_status_editor(status, index) {
		let container = document.createElement("div")
		container.className = "status_div"
		
		let createLabel = (className, text) => {
			let label = document.createElement("div")
			label.innerHTML = text
			label.className = "label " + className
			return label
		}
		
		let createPropEditor = (prop_name) => {
			let input = document.createElement("input")
			input.className = prop_name
			input.setAttribute("type", "number")
			input.value = status[prop_name]
			return input
		}
		
		let tileString = this.world_state.get_tile_at(status.location_x, status.location_y)
		
		container.appendChild(createLabel("title", index + " - " + tileString))
		container.appendChild(document.createElement("br"))
		container.appendChild(createLabel("subheading", "Location X/Y"))
		container.appendChild(createPropEditor("location_x"))
		container.appendChild(createPropEditor("location_y"))
		container.appendChild(document.createElement("br"))
		container.appendChild(createLabel("subheading", "Step X/Y"))
		container.appendChild(createPropEditor("step_x"))
		container.appendChild(createPropEditor("step_y"))
		container.appendChild(document.createElement("br"))
		container.appendChild(createLabel("subheading", "Cycle"))
		container.appendChild(createPropEditor("cycle"))
		container.appendChild(document.createElement("br"))
		container.appendChild(createLabel("param param1", "Params"))
		container.appendChild(createPropEditor("param1"))
		container.appendChild(createPropEditor("param2"))
		container.appendChild(createPropEditor("param3"))
		container.appendChild(document.createElement("br"))
		container.appendChild(createLabel("subheading", "Follower/Leader"))
		container.appendChild(createPropEditor("follower"))
		container.appendChild(createPropEditor("leader"))
		container.appendChild(document.createElement("br"))
		container.appendChild(createLabel("subheading", "Under"))
		//container.appendChild(createPropEditor("under_element_id"))
		container.appendChild(createPropEditor("under_colour"))
		container.appendChild(document.createElement("br"))
		container.appendChild(createLabel("subheading", "Code pos"))
		container.appendChild(createPropEditor("code_current_instruction"))
		
		return container
	}
	
	populate_status_editors() {
		while (this.status_elements_div.firstChild) {
			this.status_elements_div.removeChild(this.status_elements_div.firstChild)
		}
		
		for (let i in this.status_elements) {
			this.status_elements_div.appendChild(this.create_status_editor(this.status_elements[i], i))
		}
	}
	
	populate_boards_list() {
		while (this.boards_list.firstChild) {
			this.boards_list.removeChild(this.boards_list.firstChild)
		}
		for (let board of boards) {
			var board_option = document.createElement("option")
			board_option.value = ""
		}
	}
	
	render() {
		/*let current_board = this.world.boards[this.current_board_index]
		this.board_cxt.clearRect(0, 0, this.board_canvas.width, this.board_canvas.height)
		let char_w = 8
		let char_h = 14
		let tile_index = 0
		for (let y = 0; y < 25; ++y) {
			for (let x = 0; x < 60; ++x) {
				let tile = current_board.tiles[tile_index]
				let cols = zzt_colour_to_rgb(tile.colour)
				this.board_cxt.fillStyle=`rgb(${cols.fg_r},${cols.fg_g},${cols.fg_b})`
				this.board_cxt.fillRect(x * char_w, y * char_h, char_w, char_h/2)
				this.board_cxt.fillStyle=`rgb(${cols.bg_r},${cols.bg_g},${cols.bg_b})`
				this.board_cxt.fillRect(x * char_w, y * char_h + char_h/2, char_w, char_h/2)
				tile_index += 1
			}
		}*/
		let screen_chars = this.world_state.render_board()
		this.board_cxt.clearRect(0, 0, this.board_canvas.width, this.board_canvas.height)
		let char_w = 8
		let char_h = 14
		let char_index = 0
		this.board_cxt.globalCompositeOperation="source-over"
		for (let y = 0; y < 25; ++y) {
			for (let x = 0; x < 80; ++x) {
				let tile = screen_chars[char_index]
				this.board_cxt.drawImage(this.font, tile.char_code * char_w, 0, char_w, char_h, x * char_w, y * char_h, char_w, char_h)
				char_index += 1
			}
		}
		
		char_index = 0
		this.board_cxt.globalCompositeOperation="source-atop"
		for (let y = 0; y < 25; ++y) {
			for (let x = 0; x < 80; ++x) {
				let tile = screen_chars[char_index]
				this.board_cxt.fillStyle=`rgb(${tile.colour.fg_r},${tile.colour.fg_g},${tile.colour.fg_b})`
				this.board_cxt.fillRect(x * char_w, y * char_h, char_w, char_h)
				char_index += 1
			}
		}
		
		char_index = 0
		this.board_cxt.globalCompositeOperation="destination-over"
		for (let y = 0; y < 25; ++y) {
			for (let x = 0; x < 80; ++x) {
				let tile = screen_chars[char_index]
				this.board_cxt.fillStyle=`rgb(${tile.colour.bg_r},${tile.colour.bg_g},${tile.colour.bg_b})`
				this.board_cxt.fillRect(x * char_w, y * char_h, char_w, char_h)
				char_index += 1
			}
		}
	}
}
