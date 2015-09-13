#[macro_use]
extern crate log;
extern crate env_logger;
extern crate rand;
extern crate crypto;
extern crate rustc_serialize;

mod options;
mod consts;
mod connection;
mod processing;
mod common;
mod handler;
mod transport;
mod protocol;

use std::env;
use consts::common::CONF_ENV_VARIABLE;
use options::configuration::{JsonReader, ProjectOptions};
use processing::start as start_processing;
use common::helpers::Description;

fn main() {
  env_logger::init().unwrap();

  match env::var_os(CONF_ENV_VARIABLE) {
    Some(value) => {
      let path = value.into_string().unwrap();
      info!("Open configuration {}", path);
      let options = ProjectOptions::read_from_file(&path);
      info!("{} Started...", options.description());
      start_processing(&options);
    },
    None => {
      panic!(format!("Set env variable: {}", CONF_ENV_VARIABLE));
    }
  }
}
