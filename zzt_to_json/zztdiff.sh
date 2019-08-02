#! /bin/bash

temp_dir=$(mktemp -d "${TMPDIR:-/tmp/}$(basename $0).XXXXXXXXXXXX")

a_out="$temp_dir/A.json"
b_out="$temp_dir/B.json"

cargo run -- zzt json "$1" > "$a_out"
cargo run -- zzt json "$2" > "$b_out"
echo diff "$a_out" "$b_out"
diff "$a_out" "$b_out" --color=auto --unified=8 -d

rm -r "$a_out" "$b_out" "$temp_dir"
