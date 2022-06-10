extern crate contract;

extern "Rust" {
	fn __eosio_generate_abi() -> eosio_metadata::MetadataVersioned;
}

fn main() -> Result<(), std::io::Error> {
	let metadata = unsafe { __eosio_generate_abi() };
	let contents = serde_json::to_string_pretty(&metadata)?;
	print!("{}", contents);
	Ok(())
}
