
use common::helpers::Description;
use consts::common::{
  STD_LOOP_DELAY, NOTARGET_DELAY, MIN_BUFFER_SIZE, CONNECTION_FINISH_TIMEOUT};
use options::configuration::ProjectOptions;
use protocol::{
  TargetAsDigit, AnswerTargetEnum, LookAsTargetAnswerEnum};
use rustc_serialize::json;
use std::clone::Clone;
use std::collections::HashSet;
use std::error::Error;
use std::net::{SocketAddr, TcpListener};
use std::sync::{Arc, Mutex};
use std::thread;
use std::io::prelude::*;
use transport::{
  Answer, Command, ClientIdConstructor, TransportConstructor, JsonBufferCommand,
  CommandDataCreator, LockManager, AnswerWriter, ClientConnectionData,
  CuidOwner, CuidSource};


fn get_buffer_command_record(data: Vec<u8>, label: &String) -> Option<JsonBufferCommand> {
  let result: Option<JsonBufferCommand> = match String::from_utf8(data) {
    Ok(line) => match(json::decode(&line)) {
      Ok(record) => Some(record),
      Err(err) => {
        warn!("Data format error '{}' from client {}", err, label);
        None
      }
    },
    Err(err) => {
      warn!("Data decode error '{}' from {}", err, label);
      None
    }
  };
  result
}

fn prepare_command(
    command: &mut Command,
    buffer: &JsonBufferCommand,
    addr: &SocketAddr,
    options: &ProjectOptions,
    create_cuid: bool) -> bool {
  //
  if command.is_new() {
    // new command income
    command.setup(&buffer);
    if create_cuid {
      command.create_cuid(&addr, &options);
    }
  } else {
    // append data to buffer command
    command.append(&buffer);
  }
  command.is_full()
}

fn send_command_to_workers(
    buffer_size: usize,
    new_command: &Command,
    current_connection_data: &ClientConnectionData,
    command_pool: &Mutex<Vec<Command>>,
    connection_data_pool: &Mutex<Vec<ClientConnectionData>>,
    wait: bool,
    label: &String) -> usize {
  //
  let buffer_index = match command_pool.lock() {
    Ok(mut local_command_pool) => {
      let mut index = 0;
      for command in local_command_pool.iter_mut() {
        if command.lock() {
          command.copy(&new_command);
          command.unlock();
          break;
        }
        index += 1;
      }
      index
    },
    Err(err) => {
      debug!("Commad buffer for client {} lock error: {}", label, err);
      buffer_size.clone()
    }
  };
  // set client client connection data
  // for worker iteration
  if buffer_index < buffer_size {
    // temp data pool set
    loop {
      let moved = match connection_data_pool.lock() {
        Ok(mut local_connection_data_pool) => {
          local_connection_data_pool[buffer_index].clear();
          local_connection_data_pool[buffer_index].copy(&current_connection_data);
          true
        },
        Err(err) => {
          error!("Temp data pool busy for cell {}, error: {}", buffer_index, err);
          false
        }
      };
      if moved {
        break
      } else {
        thread::sleep_ms(STD_LOOP_DELAY);
      }
    }
  }
  let result = if buffer_index >= buffer_size && wait {
    thread::sleep_ms(STD_LOOP_DELAY);
    // recursively after pause
    send_command_to_workers(
      buffer_size,
      &new_command,
      &current_connection_data,
      &command_pool,
      &connection_data_pool,
      true,
      &label)
  } else {
    buffer_index
  };
  result
}

fn wait_answer(
    buffer_index: usize,
    cuid: String,
    answer_pool: &Mutex<Vec<Answer>>,
    current_connection_data: &mut ClientConnectionData,
    connection_data_pool: &Mutex<Vec<ClientConnectionData>>,
    wait: bool,
    label: &String) -> Option<Answer> {
  //
  let mut this_answer: Option<Answer> = None;
  let has_answer: bool = match answer_pool.lock() {
    Ok(mut local_answer_pool) => {
      let mut uid_equal = false;
      if local_answer_pool[buffer_index].lock() {
        let cell_uid = local_answer_pool[buffer_index].get_cuid();
        if cell_uid == cuid {
          if !local_answer_pool[buffer_index].is_shipped() {
            uid_equal = true;
            this_answer = Some(local_answer_pool[buffer_index].clone());
            // buffer cell free
            local_answer_pool[buffer_index].clear();
            debug!("Answer cell {} from worker go to client {}", buffer_index, cuid);
          } else {
            error!(
              "Answer cell {} cuid {} must send = false! From client: {}",
              buffer_index, cuid, label);
          }
        } else {
          warn!(
            "Answer cell {} wait {} exists {} from client: {}",
            buffer_index, cuid, cell_uid, label);
        }
        local_answer_pool[buffer_index].unlock();
      }
      uid_equal
    },
    Err(err) => {
      warn!(
        "Commad {} can't get pool for answer for client: {} error: {}",
        cuid, label, err);
      false
    },
  };
  let mut moved = !has_answer;
  while !moved {
    moved = match connection_data_pool.lock() {
      Ok(mut local_connection_data_pool) => {
        current_connection_data.clear();
        current_connection_data.copy(&local_connection_data_pool[buffer_index]);
        local_connection_data_pool[buffer_index].clear();
        true
      },
      Err(err) => {
        error!(
          "Temp data pool busy for cell {}, error: {}",
          buffer_index,
          err);
        false
      },
    };
    if !moved {
      thread::sleep_ms(STD_LOOP_DELAY);
    }
  }
  let result = if has_answer {
    this_answer
  } else {
    if wait {
      thread::sleep_ms(STD_LOOP_DELAY);
      wait_answer(
        buffer_index,
        cuid,
        &answer_pool,
        current_connection_data,
        &connection_data_pool,
        true,
        &label)
    } else {
      this_answer
    }
  };
  result
}

pub fn init_connection(
    options: &ProjectOptions,
    arc_command_pool: Arc<Mutex<Vec<Command>>>,
    arc_answer_pool: Arc<Mutex<Vec<Answer>>>,
    arc_connection_data_pool: Arc<Mutex<Vec<ClientConnectionData>>>,
    arc_closed_clients_set: Arc<Mutex<HashSet<String>>>) {

  let listener = match TcpListener::bind(options.socket) {
    Ok(listener) => listener,
    Err(err) => panic!(format!(
      "Can't up server on '{:?}' error: {}.", options.socket, err.description())),
  };
  let buffer_size = if options.connection_buffer_size < MIN_BUFFER_SIZE {
    MIN_BUFFER_SIZE
  } else {
    options.connection_buffer_size
  } as usize;

  for stream in listener.incoming() {
    let local_options = options.clone();
    let arc_local_command_pool = arc_command_pool.clone();
    let arc_local_answer_pool = arc_answer_pool.clone();
    let arc_local_closed_clients_set = arc_closed_clients_set.clone();
    let arc_local_connection_data_pool = arc_connection_data_pool.clone();
    // stream read thread
    match stream {
      Ok(mut stream) => {
        thread::spawn(move|| {
            let client_addr = stream.peer_addr().unwrap();
            let client_socket_label= format!("{}", client_addr);
            let command_buffer_size = local_options.command_buffer as usize;
            let mut buffer = vec![0u8; buffer_size];
            let mut close = false;
            let mut auth = false;
            let mut buffer_command = Command::new();
            let mut last_cuid: Option<String> = None;
            let mut connection_data: ClientConnectionData = ClientConnectionData::new();
            while !close {
              match stream.read(&mut buffer) {
                Ok(size) => {
                  if size > 1 {
                    let new_json_command = get_buffer_command_record(
                      (0..size).map(|index| buffer[index]).collect(), &client_socket_label);

                    let done = match new_json_command {
                      Some(json_buffer) => {
                        let need_cuid = !connection_data.has_cuid();
                        let parce_done = prepare_command(
                          &mut buffer_command,
                          &json_buffer,
                          &client_addr,
                          &local_options,
                          need_cuid);
                        
                        if need_cuid {
                          connection_data.setup_cuid(&buffer_command);
                        } else {
                          buffer_command.setup_cuid(&connection_data);
                        }
                        parce_done
                      },
                      None => false,
                    };
                    // command ready if full
                    close = !auth && buffer_command.need_auth();
                    if close {
                      warn!(
                        "Authentication failed for command: {} from {}!",
                        buffer_command.description(), client_socket_label);
                    }
                    if done && !close {
                      // move to buffer
                      let command_cuid = buffer_command.get_cuid();
                      last_cuid = Some(command_cuid.clone());
                      let buffer_index = send_command_to_workers(
                        command_buffer_size,
                        &buffer_command,
                        &connection_data,
                        &arc_local_command_pool,
                        &arc_local_connection_data_pool,
                        true,
                        &client_socket_label);

                      // clear for next command
                      buffer_command.clear();
                      // wait answer
                      let this_answer = wait_answer(
                        buffer_index,
                        command_cuid,
                        &arc_local_answer_pool,
                        &mut connection_data,
                        &arc_local_connection_data_pool,
                        true,
                        &client_socket_label);

                      match this_answer {
                        Some(answer) => {
                          // my be need close connection now
                          close = match <u32 as LookAsTargetAnswerEnum>::as_target_enum(&answer.to_u32()) {
                            AnswerTargetEnum::Quit => {
                              answer.write(&mut stream);
                              true
                            },
                            AnswerTargetEnum::Error => {
                              answer.write(&mut stream);
                              true
                            },
                            AnswerTargetEnum::Skip => {
                              // no write data for client
                              false
                            },
                            AnswerTargetEnum::WhoAreYou => {
                              auth = true;
                              !answer.write(&mut stream)
                            },
                            _ => {
                              !answer.write(&mut stream)
                            },
                          };
                        }
                        None => {
                          let cuid = match last_cuid {
                            Some(ref cuid_str) => cuid_str.clone(),
                            None => String::new(),
                          };
                          error!(
                            "No answer for command {} from client {}!",
                            cuid, client_socket_label);
                        }
                      }
                    }
                  } else {
                    close = true;
                  }
                },
                Err(err) => {
                  close = true;
                  warn!("connection {} close with error {}", client_socket_label, err);
                }
              }
              if !close {
                thread::sleep_ms(NOTARGET_DELAY);
              }
            }
            // end loop
            info!("Close connection {}", client_socket_label);
            // set flag of cloce connecion
            match last_cuid {
              Some(last_client_cuid) => {
                let end_loop_delay = NOTARGET_DELAY;
                let iter_limit: u32 = CONNECTION_FINISH_TIMEOUT * 1000 / end_loop_delay;
                // todo: delte
                info!("Iter limit {} delay {}", iter_limit, end_loop_delay);
                let mut iter_index = 0;
                loop {
                  let mut ok = match arc_local_closed_clients_set.lock() {
                    Ok(mut local_closed_clients_set) => {
                      // set closed connection
                      local_closed_clients_set.insert(last_client_cuid.clone());
                      true
                    },
                    Err(_) => {
                      iter_index += 1;
                      false
                    }
                  };
                  if iter_index >= iter_limit {
                    // fail
                    ok = true;
                    error!("Can't set flag of client {} close connection!", last_client_cuid.clone());
                  }
                  if ok {
                    break;
                  }
                }
              },
              None => {}
            }
        });
      },
      Err(err) => {
        panic!(format!(
          "Can't up server on '{:?}' error: {}.", options.socket, err.description()));
      }
    }
  }
}

// -- tests --
#[cfg(test)]
mod tests {
  extern crate rand;
  use connection::{
    get_buffer_command_record, prepare_command};
  use rand::Rng;
  use std::env;
  use std::fs::File;
  use std::io::prelude::*;
  use std::error::Error;
  use std::net::{SocketAddr, Ipv4Addr, SocketAddrV4};
  use transport::{
    Command, JsonBufferCommand, CreateTestRecord, TransportConstructor};
  use options::configuration::{JsonReader, ProjectOptions};

  fn create_options() -> ProjectOptions {
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

    let mut content = "{\"secret\": \"1234567890\",
    	\"socket\": \"100.100.100.100:8000\",
    	\"workers\": 8,
    	\"command_buffer\": 1024,
    	\"node\": \"node1\",
    	\"connection_buffer_size\": 4096}".to_string();

    file.write_all(&content.into_bytes());
    file.sync_all();

    ProjectOptions::read_from_file(&tmp_path_str)
  }

  #[test]
  fn test_buffer_command_record() {
    let label = "test".to_string();
    let json_text1 = "{\"target\": 2, \"part\": false, \"data\": \"hello\", \"cid\": \"\"}".to_string();
    match get_buffer_command_record(json_text1.into_bytes(), &label) {
      Some(record) => {
        assert!(true);
      },
      None => {
        assert!(false);
      }
    }
    let json_text2 = "{\"target\": 2, \"part\": false, \"data1\": \"hello\", \"cid\": \"\"}".to_string();
    match get_buffer_command_record(json_text2.into_bytes(), &label) {
      Some(record) => {
        assert!(false);
      },
      None => {
        assert!(true);
      }
    }
  }

  #[test]
  fn test_prepare_command_full() {
    let case = 1;
    let json_data = JsonBufferCommand::create_for_test(case);
    let mut command = Command::new();
    let addr = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(100, 100, 100, 1), 1000));
    let options = create_options();
    prepare_command(&mut command, &json_data, &addr, &options, true);
    assert!(command.check(case));
  }

  #[test]
  fn test_prepare_command_part() {
    let case = 2;
    let json_data = JsonBufferCommand::create_for_test(case);
    let mut command = Command::new();
    let addr = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(100, 100, 100, 1), 1000));
    let options = create_options();
    prepare_command(&mut command, &json_data, &addr, &options, true);
    assert!(command.check(case));
  }
}
