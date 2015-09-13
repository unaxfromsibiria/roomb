pub mod configuration {
  use common::helpers::Description;
  use consts::common::{
    MIN_COMMAND_POOL_SIZE, MIN_BUFFER_SIZE};
  use std::clone::Clone;
  use std::fs::File;
  use std::io::Read;
  use std::error::Error;
  use std::net::{SocketAddr, Ipv4Addr, SocketAddrV4};
  use rustc_serialize::json;

  pub const DEFAULT_PORT: u16 = 5882;

  pub trait JsonReader {
    fn read_from_file(file_path: &str) -> Self;
  }

  pub struct ProjectOptions {
    pub secret: String,
    pub socket: SocketAddr,
    pub workers: u32,
    pub command_buffer: u32,
    pub node: String,
    pub connection_buffer_size: u32,
  }

  impl ProjectOptions {
    fn new() -> Self {
      ProjectOptions {
        secret: String::new(),
        socket: SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), DEFAULT_PORT)),
        workers: 1,
        command_buffer: 1,
        node: String::new(),
        connection_buffer_size: MIN_BUFFER_SIZE as u32,
      }
    }
  }

  impl Clone for ProjectOptions {
    fn clone(&self) -> Self {
      ProjectOptions {
        secret: self.secret.clone(),
        socket: self.socket.clone(),
        workers: self.workers.clone(),
        command_buffer: self.command_buffer.clone(),
        node: self.node.clone(),
        connection_buffer_size: self.connection_buffer_size.clone(),
      }
    }
  }

  impl Description for ProjectOptions {
    fn description(&self) -> String {
      format!("Server {} at {}.", self.node, self.socket).to_string()
    }
  }

  #[derive(RustcDecodable, RustcEncodable)]
  struct JsonOptionRecord {
    secret: String,
    socket: String,
    workers: u32,
    node: String,
    command_buffer: u32,
    connection_buffer_size: u32,
  }

  impl JsonReader for ProjectOptions {
    fn read_from_file(file_path: &str) -> Self {
      match File::open(file_path) {
        Ok(mut file) => {
          let mut content = String::new();
          match file.read_to_string(&mut content) {
            Ok(_) => {
              debug!("Read '{}' content: {}", file_path, content);
              let json_record: JsonOptionRecord = match(json::decode(&content)) {
                Ok(record) => record,
                Err(err) => {
                  panic!(format!("File '{}' format error: {}", file_path, err.description()));
                }
              };
              let socket_str: String = json_record.socket;
              let socket = match socket_str.parse::<SocketAddr>() {
                Ok(addr_value) => addr_value,
                Err(_) => ProjectOptions::new().socket,
              };
              let min_command_pool = MIN_COMMAND_POOL_SIZE as u32;
              ProjectOptions {
                secret: json_record.secret,
                socket: socket,
                connection_buffer_size: json_record.connection_buffer_size,
                workers: json_record.workers,
                command_buffer: if json_record.command_buffer > min_command_pool {
                    json_record.command_buffer
                  } else {
                    min_command_pool
                  },
                node: json_record.node,
              }
            },
            Err(err) => {
              panic!(format!("File read error: {}", err.description()));
            }
          }
        },
        Err(err) => {
          panic!(format!("Open file '{}' error: {:?}", file_path, err));
        }
      }
    }
  }
}

#[cfg(test)]
mod tests {
  extern crate rand;
  use std::env;
  use rand::Rng;
  use std::fs::File;
  use std::io::prelude::*;
  use std::error::Error;
  use std::net::{SocketAddr, Ipv4Addr, SocketAddrV4};
  use options::configuration::{JsonReader, ProjectOptions};

  #[test]
  fn test_read_json_configuration() {
    let mut tmp_path = env::temp_dir();
    let mut rng = rand::thread_rng();
    tmp_path.push(format!("00-0{}.json", rng.gen::<i32>()));
    let tmp_path_str = tmp_path.to_str().unwrap();
    println!("Create new tmp conf file: {}", tmp_path_str);

    let mut file = match File::create(&tmp_path) {
      Err(err) => {
        panic!("Can't create tmp file: {}!", tmp_path_str);
      },
      Ok(file) => file,
    };
    // write this content
    let mut content = "{\"secret\": \"1234567890\",
    	\"socket\": \"100.100.100.100:8000\",
    	\"workers\": 8,
    	\"command_buffer\": 1024,
    	\"node\": \"node1\",
    	\"connection_buffer_size\": 4096}".to_string();
    // cargo test  -- --nocapture
    println!("{}", content);

    file.write_all(&content.into_bytes());
    file.sync_all();

    let options = ProjectOptions::read_from_file(&tmp_path_str);
    let sa = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(100, 100, 100, 100), 8000));
    assert_eq!(options.socket, sa);
    assert_eq!(options.workers, 8);
    assert_eq!(options.command_buffer, 1024);
    assert_eq!(options.connection_buffer_size, 4096);
    assert_eq!(options.secret, "1234567890".to_string());
    assert_eq!(options.node, "node1".to_string());
  }
}
