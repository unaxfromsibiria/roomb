extern crate time;
use common::helpers::{Description, get_random_digit_string};
use handler::exec::get_answer_method;
use options::configuration::ProjectOptions;
use std::clone::Clone;
use std::cmp::PartialEq;
use std::net::{SocketAddr, TcpStream};
use std::io::Write;
use protocol::{
  CommandTargetEnum, TargetAsDigit, LookAsTargetCommandEnum,
  LookAsTargetAnswerEnum, ClientGroupEnum};
use rustc_serialize::json;

// === trait ===

pub trait LockManager {
  fn lock(&mut self) -> bool;
  fn is_locked(&self) -> bool;
  fn unlock(&mut self) -> bool;
}

pub trait TransportConstructor {
  fn new() -> Self;
  fn clear(&mut self);
  fn is_new(&self) -> bool;
  fn copy(&mut self, src: &Self);
}

pub trait TransportCopy {
  fn update(&mut self, src: &Self);
}

pub trait ClientIdConstructor {
  fn create_cuid(&mut self, addr: &SocketAddr, options: &ProjectOptions);
  fn copy_cuid(&mut self, cuid: &String);
}

pub trait CuidSource {
  fn get_cuid(&self) -> String;
}

pub trait CuidOwner {
  fn setup_cuid(&mut self, src: &CuidSource) -> bool;
}

pub trait AnswerWriter {
  fn write(&self, stream: &mut TcpStream) -> bool;
}


pub trait CommandDataCreator {
  fn setup(&mut self, src: &JsonBufferCommand);
  fn append(&mut self, src: &JsonBufferCommand);
}

pub trait CommandCreationAnswer {
  fn get_answer(&self, options: &ProjectOptions, connection_data: &mut ClientConnectionData) -> Answer;
}

// === struct ===
#[derive(RustcDecodable, RustcEncodable)]
pub struct JsonBufferCommand {
  target: u32,
  part: bool,
  cid: String,
  data: String,
}

#[derive(RustcDecodable, RustcEncodable)]
pub struct JsonBufferAnswer {
  target: u32,
  cid: String,
  data: String,
}

pub struct ClientId {
  uid: String,
}

pub struct Answer {
  cuid: Option<String>,
  target: u32,
  busy: bool,
  sent: bool,
  data: String,
}

pub struct Command {
  pub cuid: Option<String>,
  target: u32,
  full: bool,
  part: bool,
  busy: bool,
  data: String,
}

pub struct ClientConnectionData {
  tmp: Vec<u8>,
  cuid: String,
  group: u32,
}

// === impl ===
impl ClientId {
  fn new() -> ClientId {
    ClientId {
      uid: String::new(),
    }
  }
}

impl Command {
  pub fn is_full(&self) -> bool {
    self.full
  }

  pub fn need_auth(&self) -> bool {
    !(
      self.target == CommandTargetEnum::SigIn.to_u32() ||
      self.target == CommandTargetEnum::Auth.to_u32() ||
      self.target == CommandTargetEnum::Quit.to_u32() ||
      self.target == CommandTargetEnum::Unknown.to_u32())
  }
}

impl Answer {
  pub fn is_shipped(&self) -> bool {
    self.sent
  }

  pub fn shipped(&mut self) {
    self.sent = true;
  }

  pub fn complete(&mut self, cuid: String) {
    self.cuid = Some(cuid);
    self.sent = false;
  }

  pub fn set_data(&mut self, data: String) {
    self.data = data;
  }
}

impl ClientConnectionData {
  pub fn new() -> Self {
    ClientConnectionData {
      cuid: String::new(),
      tmp: Vec::new(),
      group: ClientGroupEnum::Service.to_u32(),
    }  
  }

  pub fn is_exists(&self) -> bool {
    self.cuid.len() > 0 || self.tmp.len() > 0
  }
  
  pub fn has_cuid(&self) -> bool {
    warn!(" ============> {}",  self.cuid);
    self.cuid.len() > 0
  }

  pub fn is_cuid_empty(&self) -> bool {
    self.cuid.len() == 0
  }

  pub fn set_cuid(&mut self, cuid: String) {
    self.cuid = cuid;  
  }

  pub fn set_temp_data(&mut self, data: Vec<u8>) {
    self.tmp = data;
  }

  pub fn get_temp_data(&self) -> Vec<u8> {
    self.tmp.clone()
  }

  pub fn get_temp_data_as_string(&self) -> String {
    String::from_utf8(self.tmp.clone()).unwrap()
  }

  pub fn clear(&mut self) {
    self.tmp.clear();
    //self.cuid.clear();
  }

  pub fn copy(&mut self, src: &Self) {
    self.tmp = src.tmp.clone();
    self.cuid = src.cuid.clone();
  }
}
// === impl trait ===
impl PartialEq for ClientConnectionData {
  fn eq(&self, other: &ClientConnectionData) -> bool {
    self.cuid == other.cuid && self.tmp == self.tmp
  }
}

impl CuidSource for ClientConnectionData {
  fn get_cuid(&self) -> String {
    self.cuid.clone()
  }
}

impl CuidOwner for ClientConnectionData {
  fn setup_cuid(&mut self, src: &CuidSource) -> bool {
    let cuid = src.get_cuid();
    if cuid.len() > 0 {
      self.cuid = cuid;
      true
    } else {
      false
    }
  }
}

impl LookAsTargetCommandEnum for Command {
  fn as_target_enum(&self) -> CommandTargetEnum {
    <u32 as LookAsTargetCommandEnum>::as_target_enum(&self.target)
  }
}

impl CommandDataCreator for Command {
  fn setup(&mut self, src: &JsonBufferCommand) {
    self.busy = false;
    self.full = !src.part;
    self.part = src.part;
    self.data.push_str(&src.data);
    self.target = src.target;
  }

  fn append(&mut self, src: &JsonBufferCommand) {
    self.full = !src.part;
    self.part = src.part;
    self.data.push_str(&src.data);
  }
}

impl CuidOwner for Command {
  fn setup_cuid(&mut self, src: &CuidSource) -> bool {
    let cuid = src.get_cuid();
    if cuid.len() > 0 {
      self.cuid = Some(cuid);
      true
    } else {
      false
    }
  }
}

impl CuidSource for Command {
  fn get_cuid(&self) -> String {
    match self.cuid {
      Some(ref cuid) => cuid.clone(),
      None => String::new(),
    }
  }
}

impl ClientIdConstructor for ClientId {
  fn create_cuid(&mut self, addr: &SocketAddr, options: &ProjectOptions) {
    let iter_time = time::get_time();
    let line = format!(
      "{}-{}-{}-{}{}",
      options.node, addr, get_random_digit_string(4), iter_time.sec, iter_time.nsec);
    self.uid = line.to_string();
  }

  fn copy_cuid(&mut self, cuid: &String) {
    self.uid = cuid.clone();
  }
}

impl ClientIdConstructor for Command {
  fn create_cuid(&mut self, addr: &SocketAddr, options: &ProjectOptions) {
    let mut new_cuid = ClientId::new();
    new_cuid.create_cuid(&addr, &options);
    self.cuid = Some(new_cuid.uid);
  }

  fn copy_cuid(&mut self, cuid: &String) {
    self.cuid = Some(cuid.clone());
  }
}

impl ClientIdConstructor for Answer {
  fn create_cuid(&mut self, addr: &SocketAddr, options: &ProjectOptions) {
    let mut new_cuid = ClientId::new();
    new_cuid.create_cuid(addr, options);
    self.cuid = Some(new_cuid.uid);
  }

  fn copy_cuid(&mut self, cuid: &String) {
    self.cuid = Some(cuid.clone());
  }
}

impl CuidOwner for Answer {
  fn setup_cuid(&mut self, src: &CuidSource) -> bool {
    let cuid = src.get_cuid();
    if cuid.len() > 0 {
      self.cuid = Some(cuid);
      true
    } else {
      false
    }
  }
}

impl CuidSource for Answer {
  fn get_cuid(&self) -> String {
    match self.cuid {
      Some(ref cuid) => cuid.clone(),
      None => String::new(),
    }
  }
}

impl TransportConstructor for Command {
  fn new() -> Self {
    Command {
      cuid: None,
      target: CommandTargetEnum::Unknown.to_u32(),
      full: false,
      part: false,
      busy: false,
      data: String::new(),
    }
  }

  fn clear(&mut self) {
    self.busy = false;
    self.part = false;
    self.full = false;
    self.target = CommandTargetEnum::Unknown.to_u32();
    self.cuid = None;
    self.data.clear();
  }

  fn is_new(&self) -> bool {
    !self.part
  }

  fn copy(&mut self, src: &Self) {
    self.busy = false;
    self.part = false;
    self.full = true;
    self.target = src.target as u32;
    self.data = src.data.clone();
    self.cuid = src.cuid.clone();
  }
}

impl TransportConstructor for Answer {
  fn new() -> Self {
    Answer {
      cuid: None,
      target: CommandTargetEnum::Unknown.to_u32(),
      busy: false,
      sent: false,
      data: String::new(),
    }
  }

  fn clear(&mut self) {
    self.target = CommandTargetEnum::Unknown.to_u32();
    self.busy = false;
    self.sent = false;
    self.data.clear();
    self.cuid = None;
  }

  fn is_new(&self) -> bool {
    false
  }

  fn copy(&mut self, src: &Self) {
    // TODO:
  }
}

impl Clone for ClientConnectionData {
  fn clone(&self) -> Self {
    ClientConnectionData {
      cuid: self.cuid.clone(),
      tmp: self.tmp.clone(),
      group: self.group.clone(),
    }
  }
}

impl Clone for Answer {
  fn clone(&self) -> Self {
    Answer {
      cuid: self.cuid.clone(),
      target: self.target.clone(),
      busy: self.busy.clone(),
      sent: self.sent.clone(),
      data: self.data.clone(),
    }
  }
}

impl AnswerWriter for Answer {
  fn write(&self, stream: &mut TcpStream) -> bool {
    // return done - true
    match self.cuid {
      Some(ref cid) => {
        let new_data = JsonBufferAnswer {
          data: self.data.clone(),
          cid: match self.cuid {
            Some(ref cid) => cid.clone(),
            None => String::new(),
          },
          target: self.target,
        };
        match stream.write_all(json::encode(&new_data).unwrap().as_bytes()) {
          Ok(_) => true,
          Err(err) => {
            error!("Error write to client {}: {}", cid, err);
            false
          }
        }
      },
      None => {
        error!("Error no cuid! target: {}", self.target);
        false
      }
    }
  }
}

impl TargetAsDigit for Answer {
  fn to_u32(&self) -> u32 {
    self.target
  }
}

impl TransportCopy for Answer {
  fn update(&mut self, src: &Self) {
    self.cuid = match src.cuid {
      Some(ref value) => Some(value.clone()),
      None => None,
    };
    self.busy = src.busy;
    self.sent = src.sent;
    self.target = src.target;
    self.data = src.data.clone();
  }
}

impl LockManager for Command {
  fn lock(&mut self) -> bool {
    if !self.busy {
      self.busy = true;
      true      
    } else {
      false
    }
  }

  fn is_locked(&self) -> bool {
    self.busy
  }

  fn unlock(&mut self) -> bool {
    if self.busy {
      self.busy = false;
      false
    } else {
      true
    }
  }
}

impl LockManager for Answer {
  fn lock(&mut self) -> bool {
    if !self.busy {
      self.busy = true;
      true      
    } else {
      false
    }
  }

  fn is_locked(&self) -> bool {
    self.busy
  }

  fn unlock(&mut self) -> bool {
    if self.busy {
      self.busy = false;
      false
    } else {
      true
    }
  }
}

impl TransportCopy for Command {
  fn update(&mut self, src: &Self) {
    self.cuid = match src.cuid.clone() {
      Some(value) => Some(value),
      None => None,
    };
    self.full = src.full;
  }
}

impl Clone for Command {
  fn clone(&self) -> Self {
    Command {
      cuid: self.cuid.clone(),
      target: self.target.clone(),
      full: self.full.clone(),
      part: self.part.clone(),
      busy: self.busy.clone(),
      data: self.data.clone(),
    }
  }
}

impl Description for Command {
  fn description(&self) -> String {
    let st = format!(
      "<command[target:{} id:{} size:{}]>",
      <u32 as LookAsTargetCommandEnum>::as_target_enum(&self.target).description(),
      self.cuid.clone().unwrap(),
      self.data.len());
    st.to_string()
  }
}

impl Description for Answer {
  fn description(&self) -> String {
    let st = format!(
      "<answer[target:{} id:{} size:{}]>",
      <u32 as LookAsTargetAnswerEnum>::as_target_enum(&self.target).description(),
      self.cuid.clone().unwrap(),
      self.data.len());
    st.to_string()
  }
}

impl CommandCreationAnswer for Command {
  fn get_answer(&self, options: &ProjectOptions, connection_data: &mut ClientConnectionData) -> Answer {
    let (answer_target, answer_data) = get_answer_method(self.as_target_enum())(&self.data, connection_data, &options);
    Answer {
      cuid: match self.cuid {
        Some(ref cuid) => Some(cuid.clone()),
        None => {
          panic!("Try execute command without ID! Command target {}", self.target);
        },
      },
      busy: false,
      sent: false,
      data: answer_data,
      target: answer_target,
    }
  }
}

// === impl test ===
pub trait CreateTestRecord<T> {
  fn create_for_test(case: u32) -> T;
  fn check(&self, case: u32) -> bool;
}

impl CreateTestRecord<Command> for Command {
  fn create_for_test(case: u32) -> Command {
    match case {
      1 => {
        Command {
          cuid: Some("test_1".to_string()),
          target: 1,
          full: false,
          part: false,
          busy: false,
          data: String::new(),
        }
      },
      _ => {
        Command {
          cuid: Some("test_*".to_string()),
          target: 1,
          full: false,
          part: false,
          busy: false,
          data: String::new(),
        }
      }
    }
  }

  fn check(&self, case: u32) -> bool {
    match case {
      1 => {
        self.full && !self.part && !self.busy && self.target == 2
      },
      2 => {
        !self.full && self.part && !self.busy && self.target == 1
      },
      _ => false,
    }
  }
}

impl CreateTestRecord<JsonBufferCommand> for JsonBufferCommand {
  fn create_for_test(case: u32) -> JsonBufferCommand {
    match case {
      1 => {
        JsonBufferCommand {
          target: 2,
          part: false,
          cid: "test_1".to_string(),
          data: "test_1".to_string(),
        }
      },
      2 => {
        JsonBufferCommand {
          target: 1,
          part: true,
          cid: "test_0".to_string(),
          data: "test_0".to_string(),
        }
      },
      _ => {
        JsonBufferCommand {
          target: 1,
          part: true,
          cid: "test_0".to_string(),
          data: "test_0".to_string(),
        }
      }
    }
  }

  fn check(&self, case: u32) -> bool {
    true
  }
}
