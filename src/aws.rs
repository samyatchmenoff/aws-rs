extern crate time;
extern crate serialize;
extern crate url;
extern crate curl;
extern crate crypto = "rust-crypto";
extern crate xml;

use std::str;
use std::collections::HashMap;
use serialize::hex::ToHex;
use crypto::digest::Digest;
use crypto::mac::Mac;
use crypto::sha2::Sha256;
use crypto::hmac::Hmac;

pub mod util;
pub mod http;
pub mod auth;
pub mod s3;

fn hmac(key: &[u8], data: &[u8]) -> Vec<u8> {
  let mut m = Hmac::new(Sha256::new(), key);
  m.input(data);
  m.result().code().to_owned()
}

fn hash(data: &[u8]) -> Vec<u8> {
  let mut d = Sha256::new();
  d.input(data);
  let mut buf = Vec::from_elem((d.output_bits()+7)/8, 0u8);
  d.result(buf.as_mut_slice());
  buf
}

trait ToRequest {
  fn to_request(&self, credentials: auth::Credentials) -> Request;
}

struct Request {
  url: url::Url,
  method: http::Method,
  body: http::EntityBody,
  region: String,
  credentials: auth::Credentials
}

impl Request {
  fn execute(&self) -> Result<Response,String> {
    let aws_access_key_id = self.credentials.aws_access_key_id();
    let aws_secret_access_key = self.credentials.aws_secret_access_key();
    
    let now = time::now_utc();
    let full_date = now.strftime("%Y%m%dT%H%M%SZ");
    let date = now.strftime("%Y%m%d");
    
    let content_hash = hash(self.body.as_slice()).as_slice().to_hex();

    let canonical_request = format!(
      "{}\n{}\n{}\nhost:{}\nx-amz-content-sha256:{}\nx-amz-date:{}\n\nhost;x-amz-content-sha256;x-amz-date\n{}",
      self.method,
      self.url.path,
      url::query_to_str(&self.url.query),
      self.url.host, content_hash, full_date,
      content_hash
    );
    let canonical_request_hash = hash(canonical_request.as_bytes()).as_slice().to_hex();

    let string_to_sign = format!(
      "AWS4-HMAC-SHA256\n{}\n{}/{}/s3/aws4_request\n{}",
      full_date, date, self.region, canonical_request_hash
    );
    let date_key = hmac(format!("AWS4{}", aws_secret_access_key).as_bytes(), date.as_bytes());
    let date_region_key = hmac(date_key.as_slice(), self.region.as_bytes());
    let date_region_service_key = hmac(date_region_key.as_slice(), "s3".as_bytes());
    let signing_key = hmac(date_region_service_key.as_slice(), "aws4_request".as_bytes());
    let signature = hmac(signing_key.as_slice(), string_to_sign.as_bytes()).as_slice().to_hex();

    let auth_header = format!("AWS4-HMAC-SHA256 Credential={}/{}/{}/s3/aws4_request,SignedHeaders=host;x-amz-content-sha256;x-amz-date,Signature={}", aws_access_key_id, date, self.region, signature);

    let mut headers = HashMap::new();
    headers.insert("Host".to_string(), self.url.host.clone());
    headers.insert("x-amz-content-sha256".to_string(), content_hash.clone());
    headers.insert("x-amz-date".to_string(), full_date.clone());
    headers.insert("Authorization".to_string(), auth_header.clone());

    let http_request = http::Request {
      method: self.method,
      url: self.url.clone(),
      headers: headers,
      body: Some(self.body.clone())
    };

    match http_request.execute() {
      Err(e) => Err(format!("{}", e)),
      Ok(resp) => {
        match resp.status_code {
          http::status::Ok => Ok(Response { body: resp.body.unwrap_or_else(|| Vec::new()) }),
          code => Err(format!("HTTP Error: {}", code))
        }
      }
    }
  }
}

#[deriving(Show)]
pub struct Response {
  body: Vec<u8>
}

fn parse_xml(s: &str) -> Result<xml::Element,String> {
  let mut p = xml::Parser::new();
  let mut e = xml::ElementBuilder::new();
  p.feed_str(s.as_slice());
  for event in p {
    match event {
      Ok(event) => match e.push_event(event) {
        Ok(Some(e)) => return Ok(e),
        Ok(None) => (),
        Err(e) => return Err(format!("{}", e))
      },
      Err(e) => return Err(format!("XML Error: Line: {} Column: {} Msg: {}", e.line, e.col, e.msg)),
    };
  }
  return Err("Unexpected end of string".to_string());
}

impl Response {
  fn unmarshal<T: FromResponse>(&self) -> Result<T,String> {
    FromResponse::from_response(self)
  }

  fn xml_body(&self) -> Result<xml::Element,String> {
    match str::from_utf8(self.body.as_slice()) {
      Some(s) => parse_xml(s),
      None => Err("Response body is not UTF-8".to_string())
    }
  }
}

pub trait FromResponse {
  fn from_response(resp: &Response) -> Result<Self,String>;
}

