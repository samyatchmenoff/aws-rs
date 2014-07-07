use std::os;

pub enum Credentials {
  BasicCredentials(String, String)
}

impl<'r> Credentials {
  pub fn aws_access_key_id(&'r self) -> &'r str {
    match *self {
      BasicCredentials(ref key, _) => key.as_slice()
    }
  }

  pub fn aws_secret_access_key(&'r self) -> &'r str {
    match *self {
      BasicCredentials(_, ref secret) => secret.as_slice()
    }
  }
}

pub trait CredentialsProvider {
  fn get_credentials(&mut self) -> Result<Credentials,String>;
}

pub struct DefaultCredentialsProvider;

impl CredentialsProvider for DefaultCredentialsProvider {
  fn get_credentials(&mut self) -> Result<Credentials,String> {
    match (os::getenv("AWS_ACCESS_KEY_ID"), os::getenv("AWS_SECRET_ACCESS_KEY")) {
      (Some(key), Some(secret)) => Ok(BasicCredentials(key, secret)),
      _ => Err("Could not find AWS credentials".to_string())
    }
  }
}
