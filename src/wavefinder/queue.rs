// https://stackoverflow.com/a/78551675

use std::cell::RefCell;

use super::Message;

pub struct Channel {
    queue: RefCell<Vec<Message>>,
}
impl Channel {
    pub fn new() -> Self {
        Self {
            queue: RefCell::new(vec![]),
        }
    }

    fn write(&self, value: Message) {
        self.queue.borrow_mut().push(value);
    }

    fn read(&self) -> Option<Message> {
        self.queue.borrow_mut().pop()
    }
}

pub struct Writer<'a> {
    channel: &'a Channel,
}
impl<'a> Writer<'a> {
    pub fn new(channel: &'a Channel) -> Self {
        Self { channel }
    }

    pub fn write(&self, value: Message) {
        self.channel.write(value);
    }
}

pub struct Reader<'a> {
    channel: &'a Channel,
}
impl<'a> Reader<'a> {
    pub fn new(channel: &'a Channel) -> Self {
        Self { channel }
    }

    pub fn read(&self) -> Option<Message> {
        self.channel.read()
    }
}
