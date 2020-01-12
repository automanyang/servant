// -- list.rs --

pub use v3::{Pointer, List};

// --

#[allow(unused)]
pub mod v1 {
    use std::cell::RefCell;
    use std::fmt;
    use std::rc::Rc;

    // --

    impl<T: std::fmt::Debug> fmt::Debug for List<T> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            let mut r = write!(f, "[{}]", self.count);
            let mut node = self.head.clone();
            while let Some(n) = node {
                r = write!(f, "-{:?}", n.borrow());
                node = n.borrow().next.clone();
            }
            r
        }
    }

    impl<T: std::fmt::Debug> fmt::Debug for _Node<T> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(
                f,
                "({}{:?}{})",
                if self.pre.is_some() { "<" } else { "|" },
                self.value,
                if self.next.is_some() { ">" } else { "|" },
            )
        }
    }

    pub struct _Node<T> {
        value: T,
        pre: Option<Node<T>>,
        next: Option<Node<T>>,
    }
    pub type Node<T> = Rc<RefCell<_Node<T>>>;
    // unsafe impl<T> Send for Node<T> {}
    // unsafe impl<T> Sync for Node<T> {}

    pub struct List<T> {
        max_count: usize,
        count: usize,
        head: Option<Node<T>>,
        tail: Option<Node<T>>,
    }
    // unsafe，必须由程序员保证可send。
    // 我们只是在ServantRegister中使用List，而ServantRegister本身是Mutex<T>，
    // 所以是可以保证send
    // unsafe impl<T> Send for List<T> {}

    impl<T: Clone> List<T> {
        pub fn new(max_count: usize) -> Self {
            Self {
                max_count,
                count: 0,
                head: None,
                tail: None,
            }
        }
        pub fn len(&self) -> usize {
            self.count
        }
        pub fn evict_head(&mut self) -> Option<T> {
            if self.count < self.max_count {
                None
            } else {
                self.pop_head()
            }
        }
        pub fn evict_back(&mut self) -> Option<T> {
            if self.count < self.max_count {
                None
            } else {
                self.pop_back()
            }
        }
        pub fn top(&mut self, node: Node<T>) {
            if let Some(ref pre) = node.borrow().pre {
                // node不是head
                if let Some(ref next) = node.borrow().next {
                    // node不是tail
                    pre.borrow_mut().next.replace(next.clone());
                    next.borrow_mut().pre.replace(pre.clone());
                } else {
                    // node是tail，tail改为前一个
                    pre.borrow_mut().next = None;
                    self.tail = Some(pre.clone());
                }
            }
            if node.borrow().pre.is_some() {
                // node不是head，将node放在head之前
                if let Some(ref head) = self.head {
                    head.borrow_mut().pre.replace(node.clone());
                }
                node.borrow_mut().pre = None;
                node.borrow_mut().next = self.head.clone();
                // head改为node
                self.head = Some(node);
            }
        }
        pub fn bottom(&mut self, node: Node<T>) {
            if let Some(ref next) = node.borrow().next {
                // node不是tail
                if let Some(ref pre) = node.borrow().pre {
                    // node不是head，将node的pre和next连起来
                    pre.borrow_mut().next.replace(next.clone());
                    next.borrow_mut().pre.replace(pre.clone());
                } else {
                    // node是head，head改为后一个
                    next.borrow_mut().pre = None;
                    self.head = Some(next.clone());
                }
            }
            if node.borrow().next.is_some() {
                // node不是tail，将node附在tail之后
                if let Some(ref tail) = self.tail {
                    tail.borrow_mut().next.replace(node.clone());
                }
                node.borrow_mut().pre = self.tail.clone();
                node.borrow_mut().next = None;
                // 将tail设置为node
                self.tail = Some(node);
            }
        }
        pub fn remove(&mut self, node: Node<T>) {
            if let Some(ref pre) = node.borrow().pre {
                if let Some(ref next) = node.borrow().next {
                    pre.borrow_mut().next.replace(next.clone());
                    next.borrow_mut().pre.replace(pre.clone());
                } else {
                    self.pop_back();
                }
            } else {
                self.pop_head();
            }
            self.count -= 1;
        }
        pub fn push_head(&mut self, val: T) -> Node<T> {
            let node = Rc::new(RefCell::new(_Node::<T> {
                value: val,
                pre: None,
                next: self.head.clone(),
            }));
            if let Some(ref head) = self.head {
                head.borrow_mut().pre.replace(node.clone());
                self.head.replace(node.clone());
            } else {
                self.tail.replace(node.clone());
                self.head.replace(node.clone());
            }
            self.count += 1;
            node
        }
        pub fn push_back(&mut self, val: T) -> Node<T> {
            let node = Rc::new(RefCell::new(_Node::<T> {
                value: val,
                pre: self.tail.clone(),
                next: None,
            }));
            if let Some(ref tail) = self.tail {
                tail.borrow_mut().next.replace(node.clone());
                self.tail.replace(node.clone());
            } else {
                self.tail.replace(node.clone());
                self.head.replace(node.clone());
            }
            self.count += 1;
            node
        }
        pub fn pop_head(&mut self) -> Option<T> {
            if let Some(ref head) = self.head.clone() {
                if let Some(ref next) = head.borrow().next {
                    next.borrow_mut().pre = None;
                    self.head.replace(next.clone());
                } else {
                    self.head = None;
                    self.tail = None;
                }
                self.count -= 1;
                Some(head.borrow().value.clone())
            } else {
                None
            }
        }
        pub fn pop_back(&mut self) -> Option<T> {
            if let Some(ref tail) = self.tail.clone() {
                if let Some(ref pre) = tail.borrow().pre {
                    pre.borrow_mut().next = None;
                    self.tail.replace(pre.clone());
                } else {
                    self.head = None;
                    self.tail = None;
                }
                self.count -= 1;
                Some(tail.borrow().value.clone())
            } else {
                None
            }
        }
    }
}

// --

#[allow(unused)]
pub mod v2 {
    use std::fmt;
    use std::sync::Arc;

    // --

    impl<T: std::fmt::Debug> fmt::Debug for List<T> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            let mut r = write!(f, "[{}]", self.count);
            let mut node = self.head.clone();
            while let Some(n) = node {
                r = write!(f, "-{:?}", n.value);
                node = n.next.clone();
            }
            r
        }
    }

    impl<T: std::fmt::Debug> fmt::Debug for _Node<T> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(
                f,
                "({}{:?}{})",
                if self.pre.is_some() { "<" } else { "|" },
                self.value,
                if self.next.is_some() { ">" } else { "|" },
            )
        }
    }

    pub struct _Node<T> {
        value: T,
        pre: Option<Node<T>>,
        next: Option<Node<T>>,
    }
    pub type Node<T> = Arc<_Node<T>>;

    pub struct List<T> {
        max_count: usize,
        count: usize,
        head: Option<Node<T>>,
        tail: Option<Node<T>>,
    }

    impl<T: Clone> List<T> {
        pub fn new(max_count: usize) -> Self {
            Self {
                max_count,
                count: 0,
                head: None,
                tail: None,
            }
        }
        pub fn len(&self) -> usize {
            self.count
        }
        pub fn evict(&mut self) -> Option<T> {
            if self.count < self.max_count {
                None
            } else {
                self.pop()
            }
        }
        pub fn top(&mut self, mut node: Node<T>) {
            let n = Arc::get_mut(&mut node).unwrap();
            if let Some(ref mut pre) = n.pre {
                // node不是head
                if let Some(ref mut next) = n.next {
                    // node不是tail
                    Arc::get_mut(pre).unwrap().next.replace(next.clone());
                    Arc::get_mut(next).unwrap().pre.replace(pre.clone());
                } else {
                    // node是tail，tail改为前一个
                    Arc::get_mut(pre).unwrap().next = None;
                    self.tail = Some(pre.clone());
                }
            }
            if node.pre.is_some() {
                // node不是head，将node放在head之前
                if let Some(ref mut head) = self.head {
                    Arc::get_mut(head).unwrap().pre.replace(node.clone());
                }
                Arc::get_mut(&mut node).unwrap().pre = None;
                Arc::get_mut(&mut node).unwrap().next = self.head.clone();
                // head改为node
                self.head = Some(node);
            }
        }
        pub fn push(&mut self, val: T) -> Node<T> {
            let node = Arc::new(_Node::<T> {
                value: val,
                pre: None,
                next: self.head.clone(),
            });
            if let Some(ref mut head) = self.head {
                Arc::get_mut(head).unwrap().pre.replace(node.clone());
                self.head.replace(node.clone());
            } else {
                self.tail.replace(node.clone());
                self.head.replace(node.clone());
            }
            self.count += 1;
            node
        }
        pub fn pop(&mut self) -> Option<T> {
            if let Some(ref mut tail) = self.tail.clone() {
                if let Some(ref mut pre) = Arc::get_mut(tail).unwrap().pre {
                    Arc::get_mut(pre).unwrap().next = None;
                    self.tail.replace(pre.clone());
                } else {
                    self.head = None;
                    self.tail = None;
                }
                self.count -= 1;
                Some(tail.value.clone())
            } else {
                None
            }
        }
    }
}

// --

pub mod v3 {
    use std::fmt;
    use std::cell::RefCell;
    use std::rc::Rc;

    // --

    impl<T: std::fmt::Debug + Clone> fmt::Debug for List<T> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            let mut r = write!(f, "[{}/{}]", self.count, self.max_count);
            let mut node = self.head.clone();
            while let Some(n) = node {
                r = write!(f, "-{:?}", n.0.borrow().value);
                node = n.0.borrow().next.clone();
            }
            r
        }
    }

    impl<T: std::fmt::Debug> fmt::Debug for _Node<T> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(
                f,
                "({}{:?}{})",
                if self.pre.is_some() { "<" } else { "|" },
                self.value,
                if self.next.is_some() { ">" } else { "|" },
            )
        }
    }

    pub struct _Node<T> {
        value: T,
        pre: Option<Pointer<T>>,
        next: Option<Pointer<T>>,
    }

    #[derive(Clone)]
    pub struct Pointer<T>(Rc<RefCell<_Node<T>>>);
    unsafe impl<T> Send for Pointer<T> {}
    // unsafe impl<T> Sync for Pointer<T> {}

    pub struct List<T> {
        max_count: usize,
        count: usize,
        head: Option<Pointer<T>>,
        tail: Option<Pointer<T>>,
    }

    impl<T: Clone> List<T> {
        pub fn new(max_count: usize) -> Self {
            Self {
                max_count,
                count: 0,
                head: None,
                tail: None,
            }
        }
        pub fn evict(&mut self) -> Option<T> {
            if self.count < self.max_count {
                None
            } else {
                self.pop()
            }
        }
        pub fn top(&mut self, node: Pointer<T>) {
            if let Some(ref pre) = node.0.borrow().pre {
                // node不是head
                if let Some(ref next) = node.0.borrow().next {
                    // node不是tail
                    pre.0.borrow_mut().next.replace(next.clone());
                    next.0.borrow_mut().pre.replace(pre.clone());
                } else {
                    // node是tail，tail改为前一个
                    pre.0.borrow_mut().next = None;
                    self.tail = Some(pre.clone());
                }
            }
            if node.0.borrow().pre.is_some() {
                // node不是head，将node放在head之前
                if let Some(ref mut head) = self.head {
                    head.0.borrow_mut().pre.replace(node.clone());
                }
                node.0.borrow_mut().pre = None;
                node.0.borrow_mut().next = self.head.clone();
                // head改为node
                self.head = Some(node);
            }
        }
        pub fn push(&mut self, val: T) -> Pointer<T> {
            let node = Pointer(Rc::new(RefCell::new(_Node::<T> {
                value: val,
                pre: None,
                next: self.head.clone(),
            })));
            if let Some(ref mut head) = self.head {
                head.0.borrow_mut().pre.replace(node.clone());
                self.head.replace(node.clone());
            } else {
                self.tail.replace(node.clone());
                self.head.replace(node.clone());
            }
            self.count += 1;
            node
        }
        pub fn pop(&mut self) -> Option<T> {
            if let Some(ref mut tail) = self.tail.clone() {
                if let Some(ref mut pre) = tail.0.borrow_mut().pre {
                    pre.0.borrow_mut().next = None;
                    self.tail.replace(pre.clone());
                } else {
                    self.head = None;
                    self.tail = None;
                }
                self.count -= 1;
                Some(tail.0.borrow().value.clone())
            } else {
                None
            }
        }
    }
}

// --

#[cfg(test)]
mod test_v1 {
    use super::v1::*;

    #[test]
    fn test1() {
        let mut l = List::new(10);
        l.push_back(1);
        l.push_back(2);
        assert_eq!(l.pop_back(), Some(2));
        assert_eq!(l.pop_back(), Some(1));
        assert_eq!(l.pop_back(), None);
        let n1 = l.push_back(1);
        dbg!(&l);
        let n4 = l.push_head(4);
        dbg!(&l);
        let n2 = l.push_back(2);
        dbg!(&l);
        let n5 = l.push_head(5);
        dbg!(&l);
        let n3 = l.push_back(3);
        dbg!(&l);

        l.remove(n1);
        dbg!(&l);
        l.remove(n5);
        dbg!(&l);
        l.remove(n2);
        dbg!(&l);
        l.remove(n3);
        dbg!(&l);
        l.remove(n4);
        dbg!(&l);
    }

    #[test]
    fn test2() {
        let mut l = List::new(10);

        let n1 = l.push_back(1);
        dbg!(&l);
        let n4 = l.push_head(4);
        dbg!(&l);
        let n2 = l.push_back(2);
        dbg!(&l);
        let n5 = l.push_head(5);
        dbg!(&l);
        let n3 = l.push_back(3);
        dbg!(&l);

        l.top(n1);
        dbg!(&l);
        l.top(n5);
        dbg!(&l);
        l.top(n2.clone());
        dbg!(&l);
        l.top(n3.clone());
        dbg!(&l);
        l.top(n4.clone());
        dbg!(&l);

        l.bottom(n2);
        dbg!(&l);
        l.bottom(n3);
        dbg!(&l);
        l.bottom(n4);
        dbg!(&l);

        assert_eq!(Some(5), l.pop_head());
    }
}


#[cfg(test)]
mod test_v3 {
    use super::v3::*;

    #[test]
    fn test3() {
        let mut l = List::new(10);

        let n1 = l.push(1);
        dbg!(&l);
        let n4 = l.push(4);
        dbg!(&l);
        let n2 = l.push(2);
        dbg!(&l);
        let n5 = l.push(5);
        dbg!(&l);
        let n3 = l.push(3);
        dbg!(&l);

        l.top(n1);
        dbg!(&l);
        l.top(n5);
        dbg!(&l);
        l.top(n2.clone());
        dbg!(&l);
        l.top(n3.clone());
        dbg!(&l);
        l.top(n4.clone());
        dbg!(&l);

        assert_eq!(Some(1), l.pop());
    }
}