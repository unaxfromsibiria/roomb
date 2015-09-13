pub mod exec {
  use common::helpers::get_random_string;
  use consts::common::VERIFICATION_LINE_SIZE;
  use consts::messages::AUTH_FAILED_TMP;
  use crypto::sha1::Sha1;
  use crypto::digest::Digest;
  use protocol::{
    CommandTargetEnum, AnswerTargetEnum, TargetAsDigit, ClientDescription};
  use std::clone::Clone;
  use transport::{
    Answer, Command, CommandCreationAnswer, ClientConnectionData};
  use options::configuration::ProjectOptions;
  use rustc_serialize::json;

  // -- public traits --
  pub trait CommandHandle {
    fn execute(&mut self, options: &ProjectOptions, connection_data: &mut ClientConnectionData) -> Answer;
  }
  // === handlers ===
  fn answer_empty(
      client_data: &String,
      connection_data: &mut ClientConnectionData,
      options: &ProjectOptions) -> (u32, String) {
    (AnswerTargetEnum::Unknown.to_u32(), String::new())
  }

  fn answer_verification_request(
      client_data: &String,
      connection_data: &mut ClientConnectionData,
      options: &ProjectOptions) -> (u32, String) {

    let key = get_random_string(VERIFICATION_LINE_SIZE);
    connection_data.set_temp_data(key.clone().into_bytes());
    (AnswerTargetEnum::VerificationRequest.to_u32(), key)
  }

  fn answer_check_auth(
      client_data: &String,
      connection_data: &mut ClientConnectionData,
      options: &ProjectOptions) -> (u32, String) {
    let mut hasher = Sha1::new();
    let mut key_line = String::new();
    let mut client_hex = String::new();
    let mut index = 0;
    let data: Vec<char> = client_data.chars().collect();
    let secret: Vec<char> = options.secret.chars().collect();
    for ch in data {
      if index < VERIFICATION_LINE_SIZE {
        key_line.push(ch);
      } else {
        client_hex.push(ch);
      }
      index += 1;
    }
    key_line.push_str(&connection_data.get_temp_data_as_string());

    for ch in secret {
      key_line.push(ch);
    }
    hasher.input_str(&key_line);
    let hex = hasher.result_str().to_string();

    if hex == client_hex {
      (AnswerTargetEnum::WhoAreYou.to_u32(), "OK".to_string())
    } else {
      let msg = format!("{} {}", AUTH_FAILED_TMP, "Check secret key?").to_string();
      (AnswerTargetEnum::Error.to_u32(), msg)
    }
  }

  pub fn take_client_data(
      client_data: &String,
      connection_data: &mut ClientConnectionData,
      options: &ProjectOptions) -> (u32, String) {
    // data is some json
    let answer_code: u32;
    let possible_json_record: Option<ClientDescription> = match(json::decode(client_data)) {
      Ok(record) => {
        answer_code = AnswerTargetEnum::Wait.to_u32();
        Some(record)
      }
      Err(err) => {
        error!("Client data '{}' protocol error: {}", client_data, err);
        answer_code = AnswerTargetEnum::Error.to_u32();
        None
      }
    };
    let answer_data = match possible_json_record {
      Some(record) => {
        match record.get_cid() {
          //client back
          Some(cid) => {
            connection_data.set_cuid(cid);
            String::new()
          },
          None => {
            //answer_code = AnswerTargetEnum::NewCid.to_u32();
            //connection_data.get_cuid();
            String::new()
          }
        }
      },
      None => String::new(),
    };
    (answer_code, answer_data)
  }

  // === iface ===
  pub fn get_answer_method(target: CommandTargetEnum) ->
      Box<Fn(&String, &mut ClientConnectionData, &ProjectOptions) -> (u32, String)> {
    // data creator for answer
    match target {
      CommandTargetEnum::Unknown => Box::new(answer_empty),
      CommandTargetEnum::Quit => Box::new(answer_empty),
      CommandTargetEnum::SigIn => Box::new(answer_verification_request),
      CommandTargetEnum::Auth => Box::new(answer_check_auth),
      CommandTargetEnum::ClientData => Box::new(take_client_data),
    }
  }
  // === ===
  impl CommandHandle for Command {
    fn execute(&mut self, options: &ProjectOptions, connection_data: &mut ClientConnectionData) -> Answer {
      self.get_answer(&options, connection_data)
    }
  }
}
