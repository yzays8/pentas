use std::collections::VecDeque;
use std::fmt;

use terminal_size::{terminal_size, Width};

use crate::app::VerbosityLevel;

pub trait PrintableTree {
    fn print(&self, verbosity: VerbosityLevel)
    where
        Self: fmt::Display,
    {
        match verbosity {
            VerbosityLevel::Normal | VerbosityLevel::Quiet => {
                if let Some((Width(w), _)) = terminal_size() {
                    let view = self.to_string();
                    for line in view.lines() {
                        if line.len() > w as usize {
                            println!("{}...", &line[..w as usize - 3]);
                        } else {
                            println!("{}", line);
                        }
                    }
                } else {
                    println!("{}", self);
                }
            }
            VerbosityLevel::Verbose => println!("{}", self),
        }
    }

    fn print_in_chain(&mut self, verbosity: VerbosityLevel) -> &mut Self
    where
        Self: fmt::Display,
    {
        self.print(verbosity);
        println!("\n===============\n");
        self
    }
}

/// Peekable and bidirectional iterator for tokenizer/parser.
#[derive(Debug)]
pub struct TokenIterator<I>
where
    I: Clone,
{
    buf: VecDeque<I>,
    pos: usize,
}

impl<I> TokenIterator<I>
where
    I: Clone,
{
    pub fn new(arr: &[I]) -> Self {
        Self {
            buf: arr.iter().cloned().collect(),
            pos: 0,
        }
    }

    pub fn next(&mut self) -> Option<I> {
        if self.pos < self.buf.len() {
            let item = self.buf.get(self.pos).unwrap().clone();
            self.pos += 1;
            Some(item)
        } else {
            self.pos += 1;
            None
        }
    }

    #[allow(dead_code)]
    pub fn next_chunk(&mut self, size: usize) -> Vec<Option<I>> {
        let mut chunk = Vec::new();
        for _ in 0..size {
            chunk.push(self.next());
        }
        chunk
    }

    pub fn peek(&self) -> Option<&I> {
        self.buf.get(self.pos)
    }

    pub fn peek_chunk(&self, size: usize) -> Vec<Option<&I>> {
        let mut chunk = Vec::new();
        for i in 0..size {
            chunk.push(self.buf.get(self.pos + i));
        }
        chunk
    }

    pub fn rewind(&mut self, steps: usize) {
        self.pos = self.pos.saturating_sub(steps);
    }

    pub fn forward(&mut self, steps: usize) {
        self.pos = self.pos.saturating_add(steps);
    }

    pub fn get_last_consumed(&self) -> Option<&I> {
        self.buf.get(self.pos - 1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn next() {
        let arr = vec![1, 2, 3, 4, 5];
        let mut iter = TokenIterator::new(&arr);
        assert_eq!(iter.next(), Some(1));
        assert_eq!(iter.next(), Some(2));
        assert_eq!(iter.next(), Some(3));
        assert_eq!(iter.next(), Some(4));
        assert_eq!(iter.next(), Some(5));
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn peek() {
        let arr = vec![1, 2, 3, 4, 5];
        let mut iter = TokenIterator::new(&arr);
        assert_eq!(iter.peek(), Some(&1));
        assert_eq!(iter.next(), Some(1));
        assert_eq!(iter.peek(), Some(&2));
        assert_eq!(iter.next(), Some(2));
        assert_eq!(iter.peek(), Some(&3));
        assert_eq!(iter.next(), Some(3));
        assert_eq!(iter.peek(), Some(&4));
        assert_eq!(iter.next(), Some(4));
        assert_eq!(iter.peek(), Some(&5));
        assert_eq!(iter.next(), Some(5));
        assert_eq!(iter.peek(), None);
        assert_eq!(iter.next(), None);
    }
}
