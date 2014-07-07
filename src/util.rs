pub trait TryUnwrap<T,E> {
  fn try_unwrap(self, err: E) -> Result<T,E>;
}

impl<T,E> TryUnwrap<T,E> for Option<T> {
  fn try_unwrap(self, e: E) -> Result<T,E> {
    match self {
      Some(v) => Ok(v),
      None => Err(e)
    }
  }
}
