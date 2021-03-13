use crate::{print, println};
use conquer_once::spin::OnceCell;
use core::pin::Pin;
use core::task::{Context, Poll};
use crossbeam_queue::ArrayQueue;
use futures_util::stream::Stream;
use futures_util::task::AtomicWaker;
use futures_util::StreamExt;
use pc_keyboard::{layouts, DecodedKey, HandleControl, Keyboard, ScancodeSet1};

static SCANCODE_QUEUE: OnceCell<ArrayQueue<u8>> = OnceCell::uninit();
static WAKER: AtomicWaker = AtomicWaker::new();

pub struct ScancodeStream {
    _private: (),
}

impl ScancodeStream {
    pub fn new() -> Self {
        SCANCODE_QUEUE
            .try_init_once(|| ArrayQueue::new(100))
            .expect("ScancodeStream::new should only be called once");
        ScancodeStream { _private: () }
    }
}

impl Stream for ScancodeStream {
    type Item = u8;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<u8>> {
        let queue = SCANCODE_QUEUE.try_get().expect("not initialized");

        if let Some(scancode) = queue.pop() {
            return Poll::Ready(Some(scancode));
        }

        WAKER.register(&cx.waker());
        match queue.pop() {
            Some(scancode) => {
                WAKER.take();
                Poll::Ready(Some(scancode))
            }
            None => Poll::Pending,
        }
    }
}

pub(crate) fn add_scancode(scancode: u8) {
    let queue = match SCANCODE_QUEUE.try_get() {
        Ok(queue) => queue,
        Err(_) => {
            println!("WARNING: scancode queue uninitialized");
            return;
        }
    };

    if queue.push(scancode).is_ok() {
        WAKER.wake();
    } else {
        println!("WARNING: scancode queue full; dropping keyboard input");
    }
}

pub async fn print_keypresses() {
    let mut scancodes = ScancodeStream::new();
    let mut keyboard = Keyboard::new(layouts::Us104Key, ScancodeSet1, HandleControl::Ignore);

    while let Some(scancode) = scancodes.next().await {
        let key_event = match keyboard.add_byte(scancode) {
            Ok(Some(key_event)) => key_event,
            Ok(_) | Err(_) => continue,
        };

        match keyboard.process_keyevent(key_event) {
            Some(DecodedKey::Unicode(char)) => print!("{}", char),
            Some(DecodedKey::RawKey(key)) => print!("{:?}", key),
            None => continue,
        };
    }
}
