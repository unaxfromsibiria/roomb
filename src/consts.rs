pub mod common {
  pub static CONF_ENV_VARIABLE: &'static str = "CONF";
  pub static STD_LOOP_DELAY: u32 = 10;
  pub static NOTARGET_DELAY: u32 = 100;
  pub static MIN_COMMAND_POOL_SIZE: usize = 8;
  pub static MIN_BUFFER_SIZE: u32 = 2048;
  pub static CONNECTION_FINISH_TIMEOUT: u32 = 60; // sec
  pub static VERIFICATION_LINE_SIZE: usize = 128;
}

pub mod messages {
  pub static AUTH_FAILED_TMP: &'static str = "Auth filed!";
}
