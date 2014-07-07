use std::fmt;
use std::num;
use std::str;
use std::collections::HashMap;
use url;
use curl;

pub mod status;

pub enum Method {
  Head,
  Get,
  Post,
  Put,
  Delete
}

impl fmt::Show for Method {
  fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::FormatError> {
    match *self {
      Head => write!(f, "HEAD"),
      Get => write!(f, "GET"),
      Post => write!(f, "POST"),
      Put => write!(f, "PUT"),
      Delete => write!(f, "DELETE")
    }
  }
}

pub type EntityBody = Vec<u8>;

pub struct Request {
  pub method: Method,
  pub url: url::Url,
  pub headers: HashMap<String,String>,
  pub body: Option<EntityBody>
}

impl Request {
  pub fn execute(&self) -> Result<Response,String> {
    let mut handle = curl::http::handle();
    let req = match self.method {
      Get => {
        handle.get(format!("{}", self.url).as_slice())
          .headers(self.headers.iter().map(|(k,v)| (k.as_slice(), v.as_slice())))
      }
      unsupported => {
        return Err(format!("Unsupported HTTP method: {}", unsupported));
      }
    };
    let resp = try!(req.exec().map_err(|e| format!("HTTP request error: {}", e)));
    Ok(Response {
      status_code: num::FromPrimitive::from_uint(resp.get_code()).unwrap_or((status::InternalServerError)),
      body: Some(resp.get_body().to_owned())
    })
  }
}

impl fmt::Show for Request {
  fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::FormatError> {
    try!(writeln!(f, "{} {} HTTP/1.1", self.method, self.url.path));
    for (k,v) in self.headers.iter() {
      try!(writeln!(f, "{}: {}", k, v));
    }
    try!(writeln!(f, "\nBODY REDACTED..."));
    Ok(())
  }
}

pub struct Response {
  pub status_code: status::StatusCode,
  pub body: Option<EntityBody>
}

