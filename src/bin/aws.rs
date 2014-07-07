extern crate url;
extern crate aws;

use std::os;
use std::str;
use std::io;

fn print_help(prog: &str) {
  println!("Usage:");
  println!("  {} s3 ls", prog);
  println!("  {} s3 cat s3://bucket/key", prog);
}

fn cmd_s3_cat(args: &[&str]) {
  let url = url::from_str(args[0]).unwrap();
  if !url.scheme.equiv(&"s3") {
    writeln!(io::stderr(), "URL must use 's3' scheme");
    return;
  }
  let bucket_name = url.host;
  let key = url.path.path.as_slice().trim_left_chars('/');
  if key.len() == 0 {
    writeln!(io::stderr(), "Key not specified");

    return;
  }
  let mut s3 = aws::s3::S3Connection::new(aws::auth::DefaultCredentialsProvider);
  match s3.get_object(bucket_name.as_slice(), key) {
    Ok(resp) => {
      io::stdout().write(resp.content.as_slice());
    }
    Err(e) => {
      writeln!(io::stderr(), "{}", e);
    }
  };
}

fn cmd_s3_ls(args: &[&str]) {
  let mut s3 = aws::s3::S3Connection::new(aws::auth::DefaultCredentialsProvider);

  for bucket in s3.list_buckets().unwrap().buckets.iter() {
    println!("{}", bucket.name);
    let list_objects_resp = s3.list_objects(bucket.name.as_slice(), None, None, None, None).unwrap();
    for obj_summary in list_objects_resp.object_summaries.iter() {
      println!("  {}", obj_summary.key);
    }
  }
}

fn main() {
  let args = os::args();
  let mapped_args: Vec<&str> = args.as_slice().iter().map(|s| s.as_slice()).collect();
  match mapped_args.as_slice() {
    [_, "s3", "ls", .. tail] => cmd_s3_ls(tail),
    [_, "s3", "cat", .. tail] => cmd_s3_cat(tail),
    [prog, ..] => print_help(prog),
    _ => print_help("aws")
  };
}
