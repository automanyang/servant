// -- bool_to_option.rs --

pub trait BoolToOption {
    fn then_some2<T>(self, t: T) -> Option<T>;
    fn then2<T, F: FnOnce() -> T>(self, f: F) -> Option<T>;
}

impl BoolToOption for bool {
    fn then_some2<T>(self, t: T) -> Option<T> {
        if self { Some(t) } else { None }
    }
    fn then2<T, F: FnOnce() -> T>(self, f: F) -> Option<T> {
        if self { Some(f()) } else { None }
    }
}