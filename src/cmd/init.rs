// Copyright 2018-2021 Parity Technologies (UK) Ltd.
// This file is part of cargo-contract.
//
// cargo-contract is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// cargo-contract is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with cargo-contract.  If not, see <http://www.gnu.org/licenses/>.
use anyhow::{Context, Result, anyhow};
use std::{
    fs,
    path::{PathBuf, Path},
};
use structopt::StructOpt;
use handlebars::Handlebars;
use std::collections::HashMap;

/// Executes build of the smart-contract which produces a wasm binary that is ready for deploying.
///
/// It does so by invoking `cargo build` and then post processing the final binary.
#[derive(Debug, StructOpt)]
#[structopt(name = "init")]
pub struct InitCommand {
    /// Path to the Cargo.toml of the contract to build
    #[structopt(long, parse(from_os_str))]
    project_name: Option<PathBuf>,
}

impl InitCommand {
    pub fn exec(&self) -> Result<()> {
        if let Some(dir) = &self.project_name {
            fs::create_dir_all(&dir.as_path()).context(format!("Creating directory '{}'", dir.display()))?;
            let mut cargo_toml = include_str!("../../templates/contract/_Cargo.toml");
            let mut lib_rs = include_str!("../../templates/contract/lib.rs");
            let mut ignore = include_str!("../../templates/contract/.gitignore");
            
            let mut handlebars = Handlebars::new();
            handlebars
                .register_template_string("cargo_toml", cargo_toml)
                .unwrap();

            handlebars
                .register_template_string("lib_rs", lib_rs)
                .unwrap();

            let mut data = HashMap::new();
            data.insert("name", dir.to_str().unwrap());
            handlebars.render("cargo_toml", &data).unwrap();

            let cargo_file_path = dir.join("Cargo.toml");
            if cargo_file_path.exists() {
                return Err(anyhow::anyhow!("{:?} already exists!", cargo_file_path));
            }

            let lib_rs_file_path = dir.join("lib.rs");
            if lib_rs_file_path.exists() {
                return Err(anyhow::anyhow!("{:?} already exists!", lib_rs_file_path));
            }

            let ignore_file_path = dir.join(".gitignore");
            if ignore_file_path.exists() {
                return Err(anyhow::anyhow!("{:?} already exists!", ignore_file_path));
            }

            fs::write(cargo_file_path, handlebars.render("cargo_toml", &data).unwrap())?;
            fs::write(lib_rs_file_path, handlebars.render("lib_rs", &data).unwrap())?;
            fs::write(ignore_file_path, ignore)?;
        } else {
            println!("usage: cargo eosiocontract init [project_name]");
            return Ok(());
        }
        Ok(())
    }
}
