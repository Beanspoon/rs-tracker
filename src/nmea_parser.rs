use core::cell::RefCell;

use critical_section::Mutex;

#[derive(Clone, Copy)]
struct Message {
    buffer: [u8; 256],
    write_index: usize,
    ready_to_read: bool,
}

static MESSAGES: Mutex<RefCell<[Message; 10]>> = Mutex::new(RefCell::new(
    [Message {
        buffer: [0; 256],
        write_index: 0,
        ready_to_read: false,
    }; 10],
));
static WRITE_MESSAGE_INDEX: Mutex<RefCell<usize>> = Mutex::new(RefCell::new(0));

pub fn parse(chars: &[u8]) -> Result<(), &str> {
    for char in chars {
        match char {
            b'$' => critical_section::with(|cs| {
                let messages = MESSAGES.borrow_ref_mut(cs);
                let mut index = WRITE_MESSAGE_INDEX.borrow_ref_mut(cs);

                let mut completed_message = messages[*index];
                completed_message.write_index = 0;
                completed_message.ready_to_read = true;

                let mut tries = 0;
                while messages[*index].ready_to_read {
                    *index = (*index + 1) % 10;
                    if tries < 10 {
                        tries += 1;
                    } else {
                        return Err("NMEA parser ran out of message space!");
                    }
                }
                let mut message = messages[*index];
                message.buffer[message.write_index] = *char;

                Ok(())
            })?,
            _ => todo!(),
        };
    }

    Ok(())
}
