pub mod helpers {
  use rand::{thread_rng, Rng};
  use rand::distributions::{IndependentSample, Range};

  pub trait Description {
    fn description(&self) -> String;
  }

  // -----------
  fn get_random_chars(size: usize, ascii_begin: u32, ascii_end: u32) -> String {
    let range = Range::new(ascii_begin, ascii_end);
    let mut rng = thread_rng();
    String::from_utf8((0..size).map(|_| range.ind_sample(&mut rng) as u8).collect()).unwrap()
  }

  pub fn get_random_string(size: usize) -> String {
    get_random_chars(size, 48, 126)
  }

  pub fn get_random_digit_string(size: usize) -> String {
    get_random_chars(size, 48, 57)
  }
}

#[cfg(test)]
mod tests {
  use common::helpers::get_random_string;

  #[test]
  fn test_random_string() {
    let line = get_random_string(512);
    println!("New random string: {}", line);
    assert_eq!(line.len(), 512);
    let mut other_sumb = false;
    for ch in line.bytes() {
      let sumb = ch as i32;
      other_sumb &= sumb > 126 || sumb < 48;
    }
    assert!(!other_sumb);
  }
}
