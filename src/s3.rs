use std::num;
use std::result;
use url;
use super::{http, auth, Request, ToRequest, Response, FromResponse};
use super::util::TryUnwrap;
use xml;

pub struct S3Connection<C> {
  credentials_provider: C
}

impl<C: auth::CredentialsProvider> S3Connection<C> {
  pub fn new(credentials_provider: C) -> S3Connection<C> {
    S3Connection {
      credentials_provider: credentials_provider
    }
  }

  pub fn list_buckets(&mut self) -> Result<ListBucketsResponse,String> {
    let creds = try!(self.credentials_provider.get_credentials());
    let req = ListBucketsRequest;
    let raw_resp = try!(req.to_request(creds).execute());
    raw_resp.unmarshal()
  }

  pub fn list_objects(&mut self,
                      bucket_name: &str,
                      prefix: Option<&str>,
                      marker: Option<&str>,
                      delimiter: Option<&str>,
                      max_keys: Option<uint>) -> Result<ListObjectsResponse,String> {
    let creds = try!(self.credentials_provider.get_credentials());
    let req = ListObjectsRequest {
      bucket_name: bucket_name.to_string(),
      prefix: prefix.map(|x| x.to_string()),
      marker: marker.map(|x| x.to_string()),
      delimiter: delimiter.map(|x| x.to_string()),
      max_keys: max_keys
    };
    let raw_resp = try!(req.to_request(creds).execute());
    raw_resp.unmarshal()
  }

  pub fn get_object(&mut self, bucket_name: &str, key: &str) -> Result<GetObjectResponse,String> {
    let creds = try!(self.credentials_provider.get_credentials());
    let req = GetObjectRequest {
      bucket_name: bucket_name.to_string(),
      key: key.to_string()
    };
    let raw_resp = try!(req.to_request(creds).execute());
    raw_resp.unmarshal()
  }
}

struct ListBucketsRequest;

impl ToRequest for ListBucketsRequest {
  fn to_request(&self, credentials: auth::Credentials) -> Request {
    Request {
      url: url::Url {
        scheme: "https".to_string(),
        user: None,
        host: "s3.amazonaws.com".to_string(),
        port: None,
        path: url::Path {
          path: "/".to_string(),
          query: Vec::new(),
          fragment: None
        }
      },
      method: http::Get,
      body: Vec::new(),
      region: "us-east-1".to_string(),
      credentials: credentials
    }
  }
}

pub struct ListBucketsResponse {
  pub buckets: Vec<model::Bucket>
}

impl super::FromResponse for ListBucketsResponse {
  fn from_response(resp: &Response) -> Result<ListBucketsResponse,String> {
    let xml = try!(resp.xml_body());
    let ns = "http://s3.amazonaws.com/doc/2006-03-01/";
    let buckets = xml.get_child("Buckets", Some(ns)).expect("A").get_children("Bucket", Some(ns)).iter().map(|b|{
      model::Bucket { name: b.get_child("Name", Some(ns)).expect("B").content_str() }
    }).collect();
    Ok(ListBucketsResponse {
      buckets: buckets
    })
  }
}

struct ListObjectsRequest {
  bucket_name: String,
  prefix: Option<String>,
  marker: Option<String>,
  delimiter: Option<String>,
  max_keys: Option<uint>
}

impl ToRequest for ListObjectsRequest {
  fn to_request(&self, credentials: auth::Credentials) -> Request {
    let params = vec!(
      self.delimiter.clone().map(|x| ("delimiter".to_string(), x)),
      self.marker.clone().map(|x| ("marker".to_string(), x)),
      self.max_keys.clone().map(|x| ("max-keys".to_string(), format!("{}", x))),
      self.prefix.clone().map(|x| ("prefix".to_string(), x)),
    ).move_iter().filter_map(|x|x).collect();
    Request {
      url: url::Url {
        scheme: "https".to_string(),
        user: None,
        host: "s3.amazonaws.com".to_string(),
        port: None,
        path: url::Path {
          path: format!("/{}", self.bucket_name),
          query: params,
          fragment: None
        }
      },
      method: http::Get,
      body: Vec::new(),
      region: "us-east-1".to_string(),
      credentials: credentials
    }
  }
}

pub struct ListObjectsResponse {
  pub bucket_name: String,
  pub prefix: Option<String>,
  pub common_prefixes: Vec<String>,
  pub delimiter: Option<String>,
  pub marker: Option<String>,
  pub next_marker: Option<String>,
  pub max_keys: uint,
  pub truncated: bool,
  pub object_summaries: Vec<model::ObjectSummary>
}

impl super::FromResponse for ListObjectsResponse {
  fn from_response(resp: &Response) -> Result<ListObjectsResponse,String> {
    let xml = try!(resp.xml_body());
    let ns = "http://s3.amazonaws.com/doc/2006-03-01/";
    Ok(ListObjectsResponse {
      bucket_name: try!(xml.get_child("Name", Some(ns)).map(|n| n.content_str()).try_unwrap("fail".to_string())),
      prefix: xml.get_child("Prefix", Some(ns)).map(|n| n.content_str()),
      common_prefixes: Vec::new(),
      delimiter: xml.get_child("Delimiter", Some(ns)).map(|n| n.content_str()),
      marker: xml.get_child("Marker", Some(ns)).map(|n| n.content_str()),
      next_marker: xml.get_child("NextMarker", Some(ns)).map(|n| n.content_str()),
      max_keys: try!(
        xml.get_child("MaxKeys", Some(ns))
        .and_then(|n| from_str(n.content_str().as_slice()))
        .try_unwrap("MaxKeys contents invalid".to_string())
      ),
      truncated: try!(
        xml.get_child("IsTruncated", Some(ns))
        .and_then(|n| from_str(n.content_str().as_slice()))
        .try_unwrap("IsTruncated contents invalid".to_string())
      ),
      object_summaries: try!(
        result::collect(xml.get_children("Contents", Some(ns)).move_iter().map(|e| parse_object_summary(e)))
      )
    })
  }
}

struct GetObjectRequest {
  bucket_name: String,
  key: String
}

impl ToRequest for GetObjectRequest {
  fn to_request(&self, credentials: auth::Credentials) -> Request {
    Request {
      url: url::Url {
        scheme: "https".to_string(),
        user: None,
        host: "s3.amazonaws.com".to_string(),
        port: None,
        path: url::Path {
          path: format!("/{}/{}", self.bucket_name, self.key),
          query: Vec::new(),
          fragment: None
        }
      },
      method: http::Get,
      body: Vec::new(),
      region: "us-east-1".to_string(),
      credentials: credentials
    }
  }
}

pub struct GetObjectResponse {
  pub content: Vec<u8>
}

impl FromResponse for GetObjectResponse {
  fn from_response(resp: &Response) -> Result<GetObjectResponse,String> {
    Ok(GetObjectResponse { content: resp.body.clone() })
  }
}

fn parse_object_summary(xml: &xml::Element) -> Result<model::ObjectSummary,String> {
  let ns = "http://s3.amazonaws.com/doc/2006-03-01/";
  Ok(model::ObjectSummary {
    key: try!(
      xml.get_child("Key", Some(ns))
      .try_unwrap("Contents Element missing KeyElement".to_string())
      .map(|n| n.content_str())
    )
  })
}

pub mod model {
  #[deriving(Show)]
  pub struct Bucket {
    pub name: String
  }

  #[deriving(Show)]
  pub struct ObjectSummary {
    pub key: String
  }
}
