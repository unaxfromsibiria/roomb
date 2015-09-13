extern crate time;

use common::helpers::Description;
use connection::init_connection;
use consts::common::{STD_LOOP_DELAY, NOTARGET_DELAY};
use handler::exec::CommandHandle;
use options::configuration::ProjectOptions;
use transport::{
  Command, Answer, LockManager, TransportConstructor,
  TransportCopy, ClientConnectionData};
use std::clone::Clone;
use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use std::thread;
use std::io::prelude::*;

//================
pub fn start(options: &ProjectOptions) {
  let worker_count = options.workers as u32;
  let mut command_pool: Vec<Command> = Vec::new();
  let mut answer_pool: Vec<Answer> = Vec::new();
  let mut connection_data_pool: Vec<ClientConnectionData> = Vec::new();

  let command_buffer_size = options.command_buffer as usize;
  for _ in 0..command_buffer_size {
    command_pool.push(Command::new());
    answer_pool.push(Answer::new());
    connection_data_pool.push(ClientConnectionData::new());
  }

  let arc_command_pool = Arc::new(Mutex::new(command_pool));
  let arc_answer_pool = Arc::new(Mutex::new(answer_pool));
  let arc_closed_clients_set = Arc::new(Mutex::new(HashSet::new()));
  let arc_connection_data_pool = Arc::new(Mutex::new(connection_data_pool));

  for index in 0..worker_count {
    let arc_local_command_pool = arc_command_pool.clone();
    let arc_local_answer_pool = arc_answer_pool.clone();
    let arc_local_connection_data_pool = arc_connection_data_pool.clone();
    let local_options = options.clone();

    thread::spawn(move || {
      let index_num = index + 1;

      info!("{} worker started", index_num);
      let mut at_work_command: Option<Command>;
      let mut skip: bool;

      loop {
        at_work_command = None;
        skip = true;
        //debug!("step 1 {}", index_num);
        let at_cell_index = match arc_local_command_pool.lock() {
          Ok(mut local_command_pool) => {
            let mut cell_index = 0;
            for command in local_command_pool.iter_mut() {
              if command.is_full() {
                skip = false;
                if command.lock() {
                  // занимает
                  at_work_command = Some(command.clone());
                  break;
                }
              }
              cell_index += 1;
            }
            cell_index as usize
          },
          Err(err) => {
            warn!("Commad buffer in worker {} lock error: {}", index_num, err);
            thread::sleep_ms(STD_LOOP_DELAY);
            command_buffer_size
          },
        };

        if !skip && at_cell_index < command_buffer_size {
          match at_work_command {
            Some(mut work_command) => {
              let mut connection_data: ClientConnectionData;
              // get temp connection data
              loop {
                match arc_local_connection_data_pool.lock() {
                  Ok(local_connection_data_pool) => {
                    connection_data = local_connection_data_pool[at_cell_index].clone();
                    break;
                  },
                  Err(err) => {
                      warn!("Temp data pull cell {} still busy!", at_cell_index);
                      thread::sleep_ms(STD_LOOP_DELAY);
                  }
                }
              }

              loop {
                // write command cell free
                match arc_local_command_pool.lock() {
                  Ok(mut local_command_pool) => {
                    if local_command_pool[at_cell_index].unlock() {
                      local_command_pool[at_cell_index].clear();
                      info!("Command pull cell {} free.", at_cell_index);
                      break;
                    } else {
                      // wtf
                      warn!("Command pull cell {} still busy!", at_cell_index);
                      thread::sleep_ms(STD_LOOP_DELAY);
                    }
                  },
                  Err(err) => {
                    warn!("Commad buffer for free cell in worker {} lock error: {}", index_num, err);
                    thread::sleep_ms(STD_LOOP_DELAY);
                  },
                }
              }
              // end loop
              // save answer
              let answer = work_command.execute(&local_options, &mut connection_data);
              let mut answer_done = false;
              let command_description = work_command.description();
              // set temp             
              loop {
                match arc_local_connection_data_pool.lock() {
                  Ok(mut local_connection_data_pool) => {
                    local_connection_data_pool[at_cell_index].copy(&connection_data);
                    connection_data.clear();
                    break;
                  },
                  Err(err) => {
                      warn!("Temp data pull cell {} still busy!", at_cell_index);
                      thread::sleep_ms(STD_LOOP_DELAY);
                  }
                }
              }
              // free answer cell
              loop {
                match arc_local_answer_pool.lock() {
                  Ok(mut answer_pool) => {
                    for answer_cell in answer_pool.iter_mut() {
                      if !answer_cell.is_shipped() {
                        if answer_cell.lock() {
                          // set answer
                          answer_cell.update(&answer);
                          answer_cell.unlock();
                          answer_done = true;
                          break;
                        }
                      }
                    }
                  },
                  Err(err) => {
                    warn!("Answer buffer in worker {} lock error: {}", index_num, err);
                  }
                }
                if answer_done {
                  info!("Answer for {} shipped from worker {}", command_description, index_num);
                  break;
                } else {
                  warn!("Answer for {} shipping wait in worker {}", command_description, index_num);
                  thread::sleep_ms(STD_LOOP_DELAY);                  
                }
              }
            },
            None => {
              debug!("wtf");
            }
          }
        }

        if skip {
          // wait next step
          thread::sleep_ms(NOTARGET_DELAY);
          debug!("wait, all buffer free...");
        } else {
          thread::sleep_ms(STD_LOOP_DELAY);
          debug!("wait, all buffer busy...");
        }
      }

    });
  }
  // connection
  init_connection(
    &options,
    arc_command_pool,
    arc_answer_pool,
    arc_connection_data_pool,
    arc_closed_clients_set);
  // clear closed client
  loop {
    thread::sleep_ms(NOTARGET_DELAY);
  }
}
