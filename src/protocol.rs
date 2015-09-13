use common::helpers::Description;
use rustc_serialize::json;

// === data ===
pub enum CommandTargetEnum {
  SigIn,
  Quit,
  Unknown,
  Auth,
  ClientData,
}

pub enum AnswerTargetEnum {
  Skip,
  Quit,
  Unknown,
  VerificationRequest,
  Error,
  WhoAreYou,
  Wait,
}

pub enum ClientGroupEnum {
  Service,
  Server,
  Manager,
}

#[derive(RustcDecodable, RustcEncodable)]
pub struct ClientDescription {
  group: u32,
  cid: String,
}

// === trait ===

pub trait TargetAsDigit {
  fn to_u32(&self) -> u32;
}

pub trait LookAsTargetCommandEnum {
  fn as_target_enum(&self) -> CommandTargetEnum;
}

pub trait LookAsTargetAnswerEnum {
  fn as_target_enum(&self) -> AnswerTargetEnum;
}

pub trait LookAsClientGroupEnum {
  fn as_target_enum(&self) -> ClientGroupEnum;
}
// === impl ====

impl ClientDescription {
  pub fn get_cid(&self) -> Option<String> {
    if self.cid.len() > 0 {
      Some(self.cid.clone())
    } else {
      None
    }
  }
}
// === impl trait ===
impl TargetAsDigit for CommandTargetEnum {
  fn to_u32(&self) -> u32 {
    match(*self) {
      CommandTargetEnum::Unknown => 0,
      CommandTargetEnum::Quit => 1,
      CommandTargetEnum::SigIn => 2,
      CommandTargetEnum::Auth => 3,
      CommandTargetEnum::ClientData => 4,
    }
  }
}

impl TargetAsDigit for AnswerTargetEnum {
  fn to_u32(&self) -> u32 {
    match(*self) {
      AnswerTargetEnum::Unknown => 0,
      AnswerTargetEnum::Quit => 1,
      AnswerTargetEnum::Skip => 2,
      AnswerTargetEnum::VerificationRequest => 3,
      AnswerTargetEnum::Error => 4,
      AnswerTargetEnum::WhoAreYou => 5,
      AnswerTargetEnum::Wait => 6,
    }
  }
}

impl TargetAsDigit for ClientGroupEnum {
  fn to_u32(&self) -> u32 {
    match(*self) {
      ClientGroupEnum::Manager => 1,
      ClientGroupEnum::Server => 2,
      ClientGroupEnum::Service => 3,
    }
  }
}

impl Description for CommandTargetEnum {
  fn description(&self) -> String {
    match(*self) {
      CommandTargetEnum::Quit => "'quit'",
      CommandTargetEnum::Unknown => "'unknown'",
      CommandTargetEnum::SigIn => "'sigin'",
      CommandTargetEnum::Auth => "'auth'",
      CommandTargetEnum::ClientData => "'client data'",
    }.to_string()
  }
}

impl Description for AnswerTargetEnum {
  fn description(&self) -> String {
    match(*self) {
      AnswerTargetEnum::Quit => "'quit'",
      AnswerTargetEnum::Unknown => "'unknown'",
      AnswerTargetEnum::Skip => "'skip'",
      AnswerTargetEnum::VerificationRequest => "'verification request'",
      AnswerTargetEnum::Error => "'error'",
      AnswerTargetEnum::WhoAreYou => "'auth successful'",
      AnswerTargetEnum::Wait => "'wait'",
    }.to_string()
  }
}

impl Description for ClientGroupEnum {
  fn description(&self) -> String {
    match(*self) {
      ClientGroupEnum::Manager => "'manager'",
      ClientGroupEnum::Server => "'server'",
      ClientGroupEnum::Service => "'service'",
    }.to_string()
  }
}

impl LookAsTargetCommandEnum for u32 {
  fn as_target_enum(&self) -> CommandTargetEnum {
    match(*self) {
      1 => CommandTargetEnum::Quit,
      2 => CommandTargetEnum::SigIn,
      3 => CommandTargetEnum::Auth,
      _ => CommandTargetEnum::Unknown,
    }
  }
}

impl LookAsTargetAnswerEnum for u32 {
  fn as_target_enum(&self) -> AnswerTargetEnum {
    match(*self) {
      1 => AnswerTargetEnum::Quit,
      2 => AnswerTargetEnum::Skip,
      3 => AnswerTargetEnum::VerificationRequest,
      4 => AnswerTargetEnum::Error,
      5 => AnswerTargetEnum::WhoAreYou,
      _ => AnswerTargetEnum::Unknown,
    }
  }
}

impl LookAsClientGroupEnum for u32 {
  fn as_target_enum(&self) -> ClientGroupEnum {
    match(*self) {
      1 => ClientGroupEnum::Manager,
      2 => ClientGroupEnum::Server,
      _ => ClientGroupEnum::Service,
    }
  }
}
