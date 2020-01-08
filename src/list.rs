// -- list.rs --

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

pub struct List<T> {
    count: usize,
    head: Option<Node<T>>,
    tail: Option<Node<T>>,
}

impl<T: Clone> List<T> {
    pub fn new() -> Self {
        Self {
            count: 0,
            head: None,
            tail: None,
        }
    }
    pub fn len(&self) -> usize {
        self.count
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

// --

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test1() {
        let mut l = List::new();
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
        let mut l = List::new();

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
